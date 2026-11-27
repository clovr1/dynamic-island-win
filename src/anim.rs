//! Springs. Everything that moves in the island is a spring, like on the iPhone: it overshoots a little and settles.

#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub x: f32,
    pub v: f32,
    pub target: f32,
    k: f32,
    c: f32,
}

impl Spring {
    /// `response` = seconds for one "swing", `damping` = 1.0 is no overshoot, 0.6 bounces a bit.
    pub fn new(x: f32, response: f32, damping: f32) -> Spring {
        let w = std::f32::consts::TAU / response;
        Spring { x, v: 0.0, target: x, k: w * w, c: 2.0 * damping * w }
    }

    /// Change how bouncy / fast it is without losing its position.
    pub fn tune(&mut self, response: f32, damping: f32) {
        let w = std::f32::consts::TAU / response;
        self.k = w * w;
        self.c = 2.0 * damping * w;
    }

    pub fn set(&mut self, target: f32) {
        self.target = target;
    }

    pub fn jump(&mut self, x: f32) {
        self.x = x;
        self.v = 0.0;
        self.target = x;
    }

    pub fn settled(&self) -> bool {
        (self.x - self.target).abs() < 0.02 && self.v.abs() < 0.2
    }

    pub fn step(&mut self, dt: f32) {
        let dt = dt.min(0.05);
        let n = (dt / (1.0 / 240.0)).ceil().max(1.0) as i32;
        let h = dt / n as f32;
        for _ in 0..n {
            let a = self.k * (self.target - self.x) - self.c * self.v;
            self.v += a * h;
            self.x += self.v * h;
        }
        if self.settled() {
            self.x = self.target;
            self.v = 0.0;
        }
    }
}

pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

pub fn clamp01(t: f32) -> f32 {
    t.clamp(0.0, 1.0)
}

pub fn smooth(t: f32) -> f32 {
    let t = clamp01(t);
    t * t * (3.0 - 2.0 * t)
}

// ---------- CSS-style transitions ----------
// The Electron version animated with CSS `transition: width .5s cubic-bezier(...)`. These are the same thing, so the motion
// has the same speed and the same overshoot.

#[derive(Clone, Copy, Debug)]
pub struct Curve(pub f32, pub f32, pub f32, pub f32);

pub const EASE: Curve = Curve(0.25, 0.1, 0.25, 1.0);
/// the iPhone-like bounce used for most things
pub const SPRING: Curve = Curve(0.34, 1.35, 0.5, 1.0);
pub const W_EXPAND: Curve = Curve(0.3, 1.2, 0.45, 1.0);
pub const H_EXPAND: Curve = Curve(0.34, 1.4, 0.5, 1.0);
/// "sucked in": the island collapses with this
pub const SUCK: Curve = Curve(0.55, 0.0, 0.3, 1.0);

/// y for a given x on a cubic-bezier easing curve (the same maths a browser uses)
pub fn bezier(c: Curve, x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    let (x1, y1, x2, y2) = (c.0, c.1, c.2, c.3);
    let cx = 3.0 * x1;
    let bx = 3.0 * (x2 - x1) - cx;
    let ax = 1.0 - cx - bx;
    let cy = 3.0 * y1;
    let by = 3.0 * (y2 - y1) - cy;
    let ay = 1.0 - cy - by;
    let sx = |t: f32| ((ax * t + bx) * t + cx) * t;
    let dx = |t: f32| (3.0 * ax * t + 2.0 * bx) * t + cx;
    // Newton, then bisection as a safety net
    let mut t = x;
    for _ in 0..6 {
        let e = sx(t) - x;
        if e.abs() < 1e-5 {
            return ((ay * t + by) * t + cy) * t;
        }
        let d = dx(t);
        if d.abs() < 1e-6 {
            break;
        }
        t -= e / d;
    }
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    t = x;
    for _ in 0..24 {
        let e = sx(t);
        if (e - x).abs() < 1e-5 {
            break;
        }
        if e < x { lo = t } else { hi = t }
        t = (lo + hi) / 2.0;
    }
    ((ay * t + by) * t + cy) * t
}

#[derive(Clone, Copy, Debug)]
pub struct Tween {
    pub x: f32,
    pub target: f32,
    from: f32,
    start: f32,
    dur: f32,
    delay: f32,
    curve: Curve,
    done: bool,
}

impl Tween {
    pub fn new(x: f32) -> Tween {
        Tween { x, target: x, from: x, start: 0.0, dur: 0.0, delay: 0.0, curve: EASE, done: true }
    }

    /// Start moving to `target` (like changing a CSS property that has a `transition`). Same target: nothing happens.
    pub fn go(&mut self, target: f32, now: f32, dur: f32, delay: f32, curve: Curve) {
        if (target - self.target).abs() < 1e-4 {
            return;
        }
        self.from = self.x;
        self.target = target;
        self.start = now;
        self.dur = dur.max(0.001);
        self.delay = delay;
        self.curve = curve;
        self.done = false;
    }

    pub fn jump(&mut self, x: f32) {
        *self = Tween::new(x);
    }

    pub fn step(&mut self, now: f32) {
        if self.done {
            return;
        }
        let t = (now - self.start - self.delay) / self.dur;
        if t >= 1.0 {
            self.x = self.target;
            self.done = true;
        } else if t > 0.0 {
            self.x = self.from + (self.target - self.from) * bezier(self.curve, t);
        }
    }

    pub fn settled(&self) -> bool {
        self.done
    }
}
