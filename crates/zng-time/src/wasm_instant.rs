use std::{ops, time::Duration};

use wasm_bindgen::prelude::*;

#[derive(Debug, Copy, Clone)]
pub struct Instant(f64); // ms

impl PartialEq for Instant {
    fn eq(&self, other: &Instant) -> bool {
        self.0 == other.0
    }
}
impl Eq for Instant {}

impl PartialOrd for Instant {
    fn partial_cmp(&self, other: &Instant) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}
impl Ord for Instant {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.partial_cmp(&other.0).unwrap()
    }
}
impl Instant {
    pub fn now() -> Instant {
        let ms = js_sys::Reflect::get(&js_sys::global(), &JsValue::from_str("performance"))
            .expect("cannot get now, no performance object")
            .unchecked_into::<web_sys::Performance>()
            .now();
        if !ms.is_finite() {
            panic!("cannot get now, performance object returned invalid value: {ms}")
        }
        Instant(ms)
    }

    pub fn duration_since(&self, earlier: Instant) -> Duration {
        *self - earlier
    }

    pub fn checked_duration_since(&self, earlier: Instant) -> Option<Duration> {
        let ms = self.0 - earlier.0;
        if ms.is_finite() && self.0 >= earlier.0 {
            Some(Duration::from_secs_f64(ms * 1000.0))
        } else {
            None
        }
    }

    pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
        self.checked_duration_since(earlier).unwrap_or_default()
    }

    pub fn elapsed(&self) -> Duration {
        Instant::now() - *self
    }

    pub fn checked_add(&self, duration: Duration) -> Option<Instant> {
        let ms = self.0 + duration_to_ms(duration);
        if ms.is_finite() {
            Some(Self(ms))
        } else {
            None
        }
    }

    pub fn checked_sub(&self, duration: Duration) -> Option<Instant> {
        let ms = self.0 - duration_to_ms(duration);
        if ms.is_finite() {
            Some(Self(ms))
        } else {
            None
        }
    }
}

impl ops::Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, other: Duration) -> Instant {
        self.checked_add(other).expect("overflow when adding duration to instant")
    }
}

impl ops::AddAssign<Duration> for Instant {
    fn add_assign(&mut self, other: Duration) {
        *self = *self + other;
    }
}

impl ops::Sub<Duration> for Instant {
    type Output = Instant;

    fn sub(self, other: Duration) -> Instant {
        self.checked_sub(other).expect("overflow when subtracting duration from instant")
    }
}

impl ops::SubAssign<Duration> for Instant {
    fn sub_assign(&mut self, other: Duration) {
        *self = *self - other;
    }
}

impl ops::Sub<Instant> for Instant {
    type Output = Duration;

    fn sub(self, other: Instant) -> Duration {
        self.duration_since(other)
    }
}

fn duration_to_ms(d: Duration) -> f64 {
    (d.as_secs() as f64) * 1_000_000.0 + (d.subsec_nanos() as f64) / 1_000_000.0
}
