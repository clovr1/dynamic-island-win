//! Windows notifications (toasts): read through the system's own notification listener, so every app that notifies you shows up
//! with ITS OWN logo (WhatsApp, Windows Security, Mail ...), exactly the picture Windows shows on the toast.
//! Windows asks for permission once (Settings > Privacy & security > Notifications). Without it nothing is shown.

use crate::media::Shared;
use crate::ui::{ArtData, NotifData};
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use windows::core::Result;
use windows::ApplicationModel::AppDisplayInfo;
use windows::Foundation::{Size, TypedEventHandler};
use windows::Storage::Streams::DataReader;
use windows::UI::Notifications::Management::{UserNotificationListener, UserNotificationListenerAccessStatus};
use windows::UI::Notifications::UserNotificationChangedEventArgs;
use windows::UI::Notifications::{KnownNotificationBindings, NotificationKinds};
use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::WinRT::{RoInitialize, RO_INIT_MULTITHREADED};
use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

/// "a notification arrived" (WM_APP + 2)
pub const WM_NOTIF: u32 = 0x8002;

pub fn start(hwnd: isize, shared: Arc<Shared>) {
    let _ = std::thread::Builder::new().name("notify".into()).spawn(move || {
        if let Err(e) = run(hwnd, shared) {
            crate::win::log(&format!("notification thread stopped: {e}"));
        }
    });
}

fn run(hwnd: isize, shared: Arc<Shared>) -> Result<()> {
    unsafe {
        let _ = RoInitialize(RO_INIT_MULTITHREADED);
    }
    let listener = UserNotificationListener::Current()?;
    let mut seen: HashSet<u32> = HashSet::new();
    let mut first = true;
    let mut logos: HashMap<String, Option<ArtData>> = HashMap::new();
    let mut told = false;
    let mut err_logged = false;
    let mut last_len = 0usize;
    // Windows tells us the moment the list changes (so even a notification that disappears after a second is caught);
    // the 1-second poll below is the safety net
    let (tx, rx) = mpsc::channel::<()>();
    let mut subscribed = false;
    loop {
        let status = listener.RequestAccessAsync()?.get()?;
        if !told {
            crate::win::log(&format!("notifications: access status = {:?}", status));
        }
        if status != UserNotificationListenerAccessStatus::Allowed {
            if !told {
                crate::win::log("notifications: not allowed (Settings > Privacy & security > Notifications > let apps access your notifications)");
                told = true;
            }
            std::thread::sleep(Duration::from_secs(20));
            continue;
        }
        if !subscribed {
            subscribed = true;
            let tx = tx.clone();
            match listener.NotificationChanged(&TypedEventHandler::<UserNotificationListener, UserNotificationChangedEventArgs>::new(move |_, _| {
                let _ = tx.send(());
                Ok(())
            })) {
                Ok(_) => crate::win::log("notifications: change events on"),
                Err(e) => crate::win::log(&format!("notifications: change events not available ({e}); checking every second instead")),
            }
        }
        told = true;
        match poll(&listener, &mut seen, &mut first, &mut last_len, &mut logos) {
            Ok(list) => {
                if !list.is_empty() {
                    if let Ok(mut g) = shared.notifs.lock() {
                        g.extend(list);
                    }
                    unsafe {
                        let _ = PostMessageW(HWND(hwnd as *mut _), WM_NOTIF, WPARAM(0), LPARAM(0));
                    }
                }
            }
            Err(e) => {
                if !err_logged {
                    crate::win::log(&format!("notifications: {e}"));
                    err_logged = true;
                }
            }
        }
        // wait for Windows' event, or one second at most
        if rx.recv_timeout(Duration::from_millis(1000)).is_ok() {
            std::thread::sleep(Duration::from_millis(60)); // (several events usually come together)
            while rx.try_recv().is_ok() {}
        }
    }
}

fn poll(listener: &UserNotificationListener, seen: &mut HashSet<u32>, first: &mut bool, last_len: &mut usize, logos: &mut HashMap<String, Option<ArtData>>) -> Result<Vec<NotifData>> {
    let items = listener.GetNotificationsAsync(NotificationKinds::Toast)?.get()?;
    let mut out = Vec::new();
    let mut present = HashSet::new();
    for n in items {
        let Ok(id) = n.Id() else { continue };
        present.insert(id);
        if !seen.insert(id) || *first {
            continue; // already shown (or already there when the island started)
        }
        // one odd notification must never stop the others
        match build(&n, logos) {
            Ok(Some(d)) => {
                crate::win::log(&format!("notifications: new one from \"{}\" [{}] (logo: {})", d.app, d.aumid, if d.icon.is_some() { "yes" } else { "none" }));
                out.push(d);
            }
            Ok(None) => crate::win::log("notifications: ignored one (it is the island's own)"),
            Err(e) => crate::win::log(&format!("notifications: skipped one ({e})")),
        }
    }
    seen.retain(|id| present.contains(id));
    if *first {
        crate::win::log(&format!("notifications: listening ({} already in the list, not shown)", present.len()));
    } else if present.len() != *last_len {
        crate::win::log(&format!("notifications: the list now has {} item(s)", present.len()));
    }
    *last_len = present.len();
    *first = false;
    Ok(out)
}

fn build(n: &windows::UI::Notifications::UserNotification, logos: &mut HashMap<String, Option<ArtData>>) -> Result<Option<NotifData>> {
    let info = n.AppInfo()?;
    let display = info.DisplayInfo()?;
    let app = display.DisplayName().map(|s| s.to_string()).unwrap_or_default();
    if app.eq_ignore_ascii_case("dynamic island") || app.eq_ignore_ascii_case("bloop") {
        return Ok(None);
    }
    let aumid = info.AppUserModelId().map(|s| s.to_string()).unwrap_or_default();
    let mut texts = Vec::new();
    if let Ok(binding) = n.Notification()?.Visual()?.GetBinding(&KnownNotificationBindings::ToastGeneric()?) {
        for t in binding.GetTextElements()? {
            if let Ok(s) = t.Text() {
                texts.push(s.to_string());
            }
        }
    }
    let title = texts.first().cloned().unwrap_or_else(|| app.clone());
    let body = texts.iter().skip(1).cloned().collect::<Vec<_>>().join("  ");
    // a known app (or website) gets its own well-known logo, even when a browser sent the notification
    let rest = texts.iter().skip(1).cloned().collect::<Vec<_>>().join(" ");
    let icon = match crate::brands::brand_of(&app, &title, &rest).and_then(|k| crate::brands::render(k, 96)) {
        Some(b) => Some(b),
        None => logos.entry(aumid.clone()).or_insert_with(|| logo(&display)).as_ref().map(|a| ArtData { w: a.w, h: a.h, rgba: a.rgba.clone() }),
    };
    Ok(Some(NotifData { app, aumid, title, body, icon }))
}

/// The app's own logo, as Windows shows it on the toast.
fn logo(display: &AppDisplayInfo) -> Option<ArtData> {
    let r = display.GetLogo(Size { Width: 96.0, Height: 96.0 }).ok()?;
    let stream = r.OpenReadAsync().ok()?.get().ok()?;
    let size = stream.Size().ok()? as u32;
    if size == 0 || size > 8_000_000 {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0).ok()?).ok()?;
    reader.LoadAsync(size).ok()?.get().ok()?;
    let mut buf = vec![0u8; size as usize];
    reader.ReadBytes(&mut buf).ok()?;
    let img = image::load_from_memory(&buf).ok()?.to_rgba8();
    let small = image::imageops::resize(&img, 96, 96, image::imageops::FilterType::Triangle);
    Some(ArtData { w: 96, h: 96, rgba: small.into_raw() })
}
