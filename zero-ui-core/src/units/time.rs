use std::{
    fmt,
    time::{Duration, Instant},
};

use crate::impl_from_and_into_var;

/// Extension methods for initializing [`Duration`] values.
pub trait TimeUnits {
    /// Milliseconds.
    fn ms(self) -> Duration;
    /// Seconds.
    fn secs(self) -> Duration;
    /// Minutes.
    fn minutes(self) -> Duration;
    /// Hours.
    fn hours(self) -> Duration;
}
impl TimeUnits for u64 {
    fn ms(self) -> Duration {
        Duration::from_millis(self)
    }

    fn secs(self) -> Duration {
        Duration::from_secs(self)
    }

    fn minutes(self) -> Duration {
        Duration::from_secs(self * 60)
    }

    fn hours(self) -> Duration {
        Duration::from_secs(self * 60 * 60)
    }
}
impl TimeUnits for f32 {
    fn ms(self) -> Duration {
        Duration::from_secs_f32(self / 60.0)
    }

    fn secs(self) -> Duration {
        Duration::from_secs_f32(self)
    }

    fn minutes(self) -> Duration {
        Duration::from_secs_f32(self * 60.0)
    }

    fn hours(self) -> Duration {
        Duration::from_secs_f32(self * 60.0 * 60.0)
    }
}

/// Represents a timeout instant.
///
/// Timers and timeouts can be specified as an [`Instant`] in the future or as a [`Duration`] from now, both
/// of these types can be converted to this `struct`, timer related function can receive an `impl Into<Deadline>`
/// to support both methods in the same signature.
///
/// # Examples
///
/// ```
/// # use zero_ui_core::units::*;
/// fn timer(deadline: impl Into<Deadline>) { }
///
/// timer(5.secs());
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Deadline(pub Instant);
impl Deadline {
    /// New deadline from now + `dur`.
    pub fn timeout(dur: Duration) -> Self {
        Deadline(Instant::now() + dur)
    }

    /// Returns `true` if the deadline was reached.
    pub fn has_elapsed(self) -> bool {
        self.0 <= Instant::now()
    }

    /// Returns the deadline further into the past or closest to now.
    pub fn min(self, other: Deadline) -> Deadline {
        Deadline(self.0.min(other.0))
    }

    /// Returns the deadline further into the future.
    pub fn max(self, other: Deadline) -> Deadline {
        Deadline(self.0.max(other.0))
    }
}
impl fmt::Display for Deadline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let dur = self.0 - Instant::now();
        write!(f, "{:?} left", dur)
    }
}
impl fmt::Debug for Deadline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Deadline({})", self)
    }
}
impl_from_and_into_var! {
    fn from(deadline: Instant) -> Deadline {
        Deadline(deadline)
    }

    fn from(timeout: Duration) -> Deadline {
        Deadline::timeout(timeout)
    }
}
