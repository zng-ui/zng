#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Configurable instant type and service.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]

use std::{fmt, ops, time::Duration};

use parking_lot::RwLock;
use zng_app_context::app_local;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

#[cfg(target_arch = "wasm32")]
use web_time::Instant;

/// Instant service.
pub struct INSTANT;
impl INSTANT {
    /// Returns an instant corresponding to "now" or an instant configured by the app.
    ///
    /// This method can be called in non-app threads. Apps can override this time in app threads,
    /// by default the time is *paused* for each widget OP pass so that all widgets observe the same
    /// time on the same pass, you can use [`mode`](Self::mode) to check how `now` updates and you
    /// can use the `APP.pause_time_for_update` variable to disable pausing.
    pub fn now(&self) -> DInstant {
        if zng_app_context::LocalContext::current_app().is_some()
            && let Some(now) = INSTANT_SV.read().now
        {
            return now;
        }
        DInstant(self.epoch().elapsed())
    }

    /// Instant of first usage of the [`INSTANT`] service in the process, minus one day.
    ///
    /// # Panics
    ///
    /// Panics if called in a non-app thread.
    pub fn epoch(&self) -> Instant {
        if let Some(t) = *EPOCH.read() {
            return t;
        }
        *EPOCH.write().get_or_insert_with(|| {
            let mut now = Instant::now();
            // some CI machines (Github Windows) fail to subtract 1 day.
            for t in [60 * 60 * 24, 60 * 60, 60 * 30, 60 * 15, 60 * 10, 60] {
                if let Some(t) = now.checked_sub(Duration::from_secs(t)) {
                    now = t;
                    break;
                }
            }
            now
        })
    }

    /// Defines how the `now` value updates.
    ///
    /// # Panics
    ///
    /// Panics if called in a non-app thread.
    pub fn mode(&self) -> InstantMode {
        INSTANT_SV.read().mode
    }
}

/// App control of the [`INSTANT`] service in an app context.
#[expect(non_camel_case_types)]
pub struct INSTANT_APP;
impl INSTANT_APP {
    /// Set how the app controls the time.
    ///
    /// If mode is set to [`InstantMode::Now`] the custom now is unset.
    pub fn set_mode(&self, mode: InstantMode) {
        let mut sv = INSTANT_SV.write();
        sv.mode = mode;
        if let InstantMode::Now = mode {
            sv.now = None;
        }
    }

    /// Set the [`INSTANT.now`] for the app threads.
    ///
    /// # Panics
    ///
    /// Panics if the mode is [`InstantMode::Now`].
    ///
    /// [`INSTANT.now`]: INSTANT::now
    pub fn set_now(&self, now: DInstant) {
        let mut sv = INSTANT_SV.write();
        if let InstantMode::Now = sv.mode {
            panic!("cannot set now with `TimeMode::Now`");
        }
        sv.now = Some(now);
    }

    /// Set the [`INSTANT.now`] for the app threads to the current time plus `advance`.
    ///
    /// # Panics
    ///
    /// Panics if the mode is not [`InstantMode::Manual`].
    ///
    /// [`INSTANT.now`]: INSTANT::now
    pub fn advance_now(&self, advance: Duration) {
        let mut sv = INSTANT_SV.write();
        if let InstantMode::Manual = sv.mode {
            *sv.now.get_or_insert_with(|| DInstant(INSTANT.epoch().elapsed())) += advance;
        } else {
            panic!("cannot advance now, not `InstantMode::Manual`");
        }
    }

    /// Unset the custom now value.
    pub fn unset_now(&self) {
        INSTANT_SV.write().now = None;
    }

    /// Gets the custom now value.
    ///
    /// This value is returned by [`INSTANT.now`] if set.
    ///
    /// [`INSTANT.now`]: INSTANT::now
    pub fn custom_now(&self) -> Option<DInstant> {
        INSTANT_SV.read().now
    }

    /// If mode is [`InstantMode::UpdatePaused`] sets the app custom_now to the current time and returns
    /// an object that unsets the custom now on drop.
    pub fn pause_for_update(&self) -> Option<InstantUpdatePause> {
        let mut sv = INSTANT_SV.write();
        match sv.mode {
            InstantMode::UpdatePaused => {
                let now = DInstant(INSTANT.epoch().elapsed());
                sv.now = Some(now);
                Some(InstantUpdatePause { now })
            }
            _ => None,
        }
    }
}

/// Unset now on drop.
///
/// The time is only unset if it is still set to the same pause time.
#[must_use = "unset_now on drop"]
pub struct InstantUpdatePause {
    now: DInstant,
}
impl Drop for InstantUpdatePause {
    fn drop(&mut self) {
        let mut sv = INSTANT_SV.write();
        if sv.now == Some(self.now) {
            sv.now = None;
        }
    }
}

/// Duration elapsed since an epoch.
///
/// By default this is the duration elapsed since the first usage of [`INSTANT`] in the process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DInstant(Duration);
impl DInstant {
    /// Returns the amount of time elapsed since this instant.
    pub fn elapsed(self) -> Duration {
        INSTANT.now().0 - self.0
    }

    /// Returns the amount of time elapsed from another instant to this one,
    /// or zero duration if that instant is later than this one.
    pub fn duration_since(self, earlier: DInstant) -> Duration {
        self.0 - earlier.0
    }

    /// Returns `Some(t)` where t is the time `self + duration` if t can be represented.
    pub fn checked_add(&self, duration: Duration) -> Option<DInstant> {
        self.0.checked_add(duration).map(Self)
    }

    /// Returns `Some(t)`` where t is the time `self - duration` if `duration` greater then the elapsed time
    /// since the process start.
    pub fn checked_sub(self, duration: Duration) -> Option<DInstant> {
        self.0.checked_sub(duration).map(Self)
    }

    /// Returns the amount of time elapsed from another instant to this one, or None if that instant is later than this one.
    pub fn checked_duration_since(&self, earlier: DInstant) -> Option<Duration> {
        self.0.checked_sub(earlier.0)
    }

    /// Returns the amount of time elapsed from another instant to this one, or zero duration if that instant is later than this one.
    pub fn saturating_duration_since(&self, earlier: DInstant) -> Duration {
        self.0.saturating_sub(earlier.0)
    }

    /// Earliest instant.
    pub const EPOCH: DInstant = DInstant(Duration::ZERO);

    /// The maximum representable instant.
    pub const MAX: DInstant = DInstant(Duration::MAX);
}
impl ops::Add<Duration> for DInstant {
    type Output = Self;

    fn add(self, rhs: Duration) -> Self {
        Self(self.0.saturating_add(rhs))
    }
}
impl ops::AddAssign<Duration> for DInstant {
    fn add_assign(&mut self, rhs: Duration) {
        self.0 = self.0.saturating_add(rhs);
    }
}
impl ops::Sub<Duration> for DInstant {
    type Output = Self;

    fn sub(self, rhs: Duration) -> Self {
        Self(self.0.saturating_sub(rhs))
    }
}
impl ops::SubAssign<Duration> for DInstant {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 = self.0.saturating_sub(rhs);
    }
}
impl ops::Sub for DInstant {
    type Output = Duration;

    fn sub(self, rhs: Self) -> Self::Output {
        self.0.saturating_sub(rhs.0)
    }
}
impl From<DInstant> for Instant {
    fn from(t: DInstant) -> Self {
        INSTANT.epoch() + t.0
    }
}

/// Defines how the [`INSTANT.now`] value updates in the app.
///
/// [`INSTANT.now`]: INSTANT::now
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstantMode {
    /// Calls during an update pass (or layout, render, etc.) read the same time.
    /// Other calls to `now` resamples the time.
    UpdatePaused,
    /// Every call to `now` resamples the time.
    Now,
    /// Time is controlled by the app.
    Manual,
}

static EPOCH: RwLock<Option<Instant>> = RwLock::new(None);

app_local! {
    static INSTANT_SV: InstantService = const {
        InstantService {
            mode: InstantMode::UpdatePaused,
            now: None,
        }
    };
}

struct InstantService {
    mode: InstantMode,
    now: Option<DInstant>,
}

/// Represents a timeout instant.
///
/// Deadlines and timeouts can be specified as a [`DInstant`] in the future or as a [`Duration`] from now, both
/// of these types can be converted to this `struct`.
///
/// # Examples
///
/// In the example below the timer function accepts `Deadline`, `DInstant` and `Duration` inputs.
///
/// ```
/// # use zng_time::*;
/// # trait TimeUnits { fn secs(self) -> std::time::Duration where Self: Sized { std::time::Duration::ZERO } }
/// # impl TimeUnits for i32 { }
/// fn timer(deadline: impl Into<Deadline>) {
///     let deadline = deadline.into();
///     // ..
/// }
///
/// timer(5.secs());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Deadline(pub DInstant);
impl Deadline {
    /// New deadline from now + `dur`.
    pub fn timeout(dur: Duration) -> Self {
        Deadline(INSTANT.now() + dur)
    }

    /// Returns `true` if the deadline was reached.
    pub fn has_elapsed(self) -> bool {
        self.0 <= INSTANT.now()
    }

    /// Returns the time left until the deadline is reached.
    pub fn time_left(self) -> Option<Duration> {
        self.0.checked_duration_since(INSTANT.now())
    }

    /// Returns the deadline further into the past or closest to now.
    pub fn min(self, other: Deadline) -> Deadline {
        Deadline(self.0.min(other.0))
    }

    /// Returns the deadline further into the future.
    pub fn max(self, other: Deadline) -> Deadline {
        Deadline(self.0.max(other.0))
    }

    /// Deadline that is always elapsed.
    pub const ELAPSED: Deadline = Deadline(DInstant::EPOCH);

    /// Deadline that is practically never reached.
    pub const MAX: Deadline = Deadline(DInstant::MAX);
}
impl fmt::Display for Deadline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dur = self.0 - INSTANT.now();
        write!(f, "{dur:?} left")
    }
}
impl fmt::Debug for Deadline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deadline({self})")
    }
}
impl From<DInstant> for Deadline {
    fn from(value: DInstant) -> Self {
        Deadline(value)
    }
}
impl From<Duration> for Deadline {
    fn from(value: Duration) -> Self {
        Deadline::timeout(value)
    }
}
impl ops::Add<Duration> for Deadline {
    type Output = Self;

    fn add(mut self, rhs: Duration) -> Self {
        self.0 += rhs;
        self
    }
}
impl ops::AddAssign<Duration> for Deadline {
    fn add_assign(&mut self, rhs: Duration) {
        self.0 += rhs;
    }
}
impl ops::Sub<Duration> for Deadline {
    type Output = Self;

    fn sub(mut self, rhs: Duration) -> Self {
        self.0 -= rhs;
        self
    }
}
impl ops::SubAssign<Duration> for Deadline {
    fn sub_assign(&mut self, rhs: Duration) {
        self.0 -= rhs;
    }
}
