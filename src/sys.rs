//! Watches the quick settings of Windows (Wi-Fi, Bluetooth, airplane mode, Night light, hotspot, energy saver), the battery and Do Not
//! Disturb, and tells the window when one of them CHANGES (the first reading is only remembered). Everything is read-only.

use crate::media::Shared;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::Arc;
use std::time::{Duration, Instant};
use windows::core::{w, PCSTR, PCWSTR};
use windows::Devices::Radios::{Radio, RadioKind, RadioState};
use windows::Networking::Connectivity::NetworkInformation;
use windows::Networking::NetworkOperators::{NetworkOperatorTetheringManager, TetheringOperationalState};
use windows::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, WPARAM};
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::Registry::*;
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// "something about the system changed" (WM_APP + 7)
pub const WM_SYS: u32 = 0x8007;

pub enum SysMsg {
    Toggle(String, bool),
    Battery { label: String, level: i32, charging: bool, low: bool },
    /// Do Not Disturb; `first`: the very first reading (only remembered, no pill)
    Dnd { on: bool, first: bool },
}

pub fn start(hwnd: isize, shared: Arc<Shared>) {
    let _ = std::thread::Builder::new().name("system".into()).spawn(move || run(hwnd, shared));
}

struct Watch {
    hwnd: isize,
    shared: Arc<Shared>,
    state: HashMap<&'static str, bool>,
}

impl Watch {
    fn send(&self, m: SysMsg) {
        if let Ok(mut g) = self.shared.sys.lock() {
            g.push(m);
        }
        unsafe {
            let _ = PostMessageW(HWND(self.hwnd as *mut _), WM_SYS, WPARAM(0), LPARAM(0));
        }
    }

    /// The first reading is only remembered; every later change is reported.
    fn set(&mut self, key: &'static str, on: bool, quiet: bool) {
        match self.state.get(key) {
            None => {
                self.state.insert(key, on);
            }
            Some(&old) if old == on => {}
            Some(_) => {
                self.state.insert(key, on);
                if !quiet {
                    crate::win::log(&format!("system: {key} -> {}", if on { "on" } else { "off" }));
                    self.send(SysMsg::Toggle(key.to_string(), on));
                }
            }
        }
    }
}

fn reg_dword(root: HKEY, sub: PCWSTR, name: PCWSTR) -> Option<u32> {
    unsafe {
        let mut v = 0u32;
        let mut n = 4u32;
        let r = RegGetValueW(root, sub, name, RRF_RT_REG_DWORD, None, Some(&mut v as *mut u32 as *mut c_void), Some(&mut n));
        (r == ERROR_SUCCESS).then_some(v)
    }
}

fn reg_binary(root: HKEY, sub: &str, name: PCWSTR) -> Option<Vec<u8>> {
    unsafe {
        let s: Vec<u16> = sub.encode_utf16().chain(std::iter::once(0)).collect();
        let mut n = 0u32;
        if RegGetValueW(root, PCWSTR(s.as_ptr()), name, RRF_RT_REG_BINARY, None, None, Some(&mut n)) != ERROR_SUCCESS || n == 0 || n > 4096 {
            return None;
        }
        let mut buf = vec![0u8; n as usize];
        if RegGetValueW(root, PCWSTR(s.as_ptr()), name, RRF_RT_REG_BINARY, None, Some(buf.as_mut_ptr() as *mut c_void), Some(&mut n)) != ERROR_SUCCESS {
            return None;
        }
        buf.truncate(n as usize);
        Some(buf)
    }
}

/// Windows keeps "quiet hours" (Do Not Disturb) in an undocumented notification state: 1 = on.
fn read_dnd() -> Option<u32> {
    type Query = unsafe extern "system" fn(*const [u32; 2], *const c_void, *const c_void, *mut u32, *mut c_void, *mut u32) -> i32;
    unsafe {
        let nt = GetModuleHandleW(w!("ntdll.dll")).ok()?;
        let f = GetProcAddress(nt, PCSTR(b"NtQueryWnfStateData\0".as_ptr()))?;
        let q: Query = std::mem::transmute(f);
        let name = [0xA3BF_1C75u32, 0x0D83_063E];
        let mut stamp = 0u32;
        let mut buf = [0u32; 4];
        let mut size = 16u32;
        let st = q(&name, std::ptr::null(), std::ptr::null(), &mut stamp, buf.as_mut_ptr() as *mut c_void, &mut size);
        (st >= 0 && size >= 4).then_some(buf[0])
    }
}

const NL_PATHS: [&str; 2] = [
    "Software\\Microsoft\\Windows\\CurrentVersion\\CloudStore\\Store\\DefaultAccount\\Current\\default$windows.data.bluelightreduction.bluelightreductionstate\\windows.data.bluelightreduction.bluelightreductionstate",
    "Software\\Microsoft\\Windows\\CurrentVersion\\CloudStore\\Store\\Cache\\DefaultAccount\\$$windows.data.bluelightreduction.bluelightreductionstate\\Current",
];

fn run(hwnd: isize, shared: Arc<Shared>) {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let mut w = Watch { hwnd, shared, state: HashMap::new() };
    let mut tick = 0u32;
    let mut airplane_until: Option<Instant> = None;
    let mut dnd_first = true;
    let mut dnd_last: Option<bool> = None;
    let mut batt_prev: Option<bool> = None;
    let mut low_shown = false;
    let mut nl_prev: Option<Vec<u8>> = None;
    let mut nl_on = false;
    loop {
        tick += 1;
        // airplane mode switches Wi-Fi / Bluetooth / cellular too: then only the airplane pill is wanted
        if let Some(v) = reg_dword(HKEY_LOCAL_MACHINE, w!("SYSTEM\\CurrentControlSet\\Control\\RadioManagement"), w!("SystemRadioState")) {
            let on = v == 1;
            if w.state.get("airplane").map(|o| *o != on).unwrap_or(false) {
                airplane_until = Some(Instant::now() + Duration::from_secs(5));
            }
            w.set("airplane", on, false);
        }
        let quiet = airplane_until.map(|t| Instant::now() < t).unwrap_or(false);

        if tick % 2 == 0 {
            if let Ok(op) = Radio::GetRadiosAsync() {
                if let Ok(list) = op.get() {
                    let mut kinds: HashMap<&'static str, bool> = HashMap::new();
                    for r in list {
                        let key = match r.Kind() {
                            Ok(RadioKind::WiFi) => "wifi",
                            Ok(RadioKind::Bluetooth) => "bluetooth",
                            Ok(RadioKind::MobileBroadband) => "cellular",
                            _ => continue,
                        };
                        let on = r.State().map(|s| s == RadioState::On).unwrap_or(false);
                        let e = kinds.entry(key).or_insert(false);
                        *e = *e || on;
                    }
                    for (k, on) in kinds {
                        w.set(k, on, quiet);
                    }
                }
            }
        }

        // battery + energy saver
        unsafe {
            let mut st = SYSTEM_POWER_STATUS::default();
            if GetSystemPowerStatus(&mut st).is_ok() {
                w.set("energy", st.SystemStatusFlag == 1, false);
                if st.BatteryFlag & 128 == 0 && st.BatteryLifePercent <= 100 {
                    let level = st.BatteryLifePercent as i32;
                    let charging = st.ACLineStatus == 1;
                    if let Some(prev) = batt_prev {
                        if prev != charging {
                            w.send(SysMsg::Battery { label: if charging { "Charging" } else { "Unplugged" }.into(), level, charging, low: false });
                        }
                    }
                    batt_prev = Some(charging);
                    if level <= 20 && !charging && !low_shown {
                        low_shown = true;
                        w.send(SysMsg::Battery { label: "Low Battery".into(), level, charging, low: true });
                    }
                    if level > 20 || charging {
                        low_shown = false;
                    }
                }
            }
        }

        // Do Not Disturb
        if let Some(v) = read_dnd() {
            let on = v == 1;
            if dnd_last != Some(on) {
                dnd_last = Some(on);
                w.send(SysMsg::Dnd { on, first: dnd_first });
                dnd_first = false;
            }
        }

        // Night light: a small binary blob in the registry; every change of it is one switch
        for p in NL_PATHS {
            if let Some(d) = reg_binary(HKEY_CURRENT_USER, p, w!("Data")) {
                match &nl_prev {
                    None => {
                        nl_on = d.len() >= 43 || (d.len() > 19 && d[18] == 0x15);
                        w.set("nightlight", nl_on, false);
                    }
                    Some(prev) if *prev != d => {
                        let on = if prev.len() != d.len() { d.len() > prev.len() } else { !nl_on };
                        nl_on = on;
                        w.set("nightlight", on, false);
                    }
                    _ => {}
                }
                nl_prev = Some(d);
                break;
            }
        }

        // hotspot: a heavier call, every third round
        if tick % 3 == 0 {
            if let Ok(prof) = NetworkInformation::GetInternetConnectionProfile() {
                if let Ok(tm) = NetworkOperatorTetheringManager::CreateFromConnectionProfile(&prof) {
                    if let Ok(s) = tm.TetheringOperationalState() {
                        w.set("hotspot", s == TetheringOperationalState::On, false);
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(1000));
    }
}
