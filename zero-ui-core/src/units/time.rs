use std::time::Duration;

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
    #[inline]
    fn ms(self) -> Duration {
        Duration::from_millis(self)
    }

    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs(self)
    }

    #[inline]
    fn minutes(self) -> Duration {
        Duration::from_secs(self * 60)
    }

    #[inline]
    fn hours(self) -> Duration {
        Duration::from_secs(self * 60 * 60)
    }
}
impl TimeUnits for f32 {
    #[inline]
    fn ms(self) -> Duration {
        Duration::from_secs_f32(self / 60.0)
    }

    #[inline]
    fn secs(self) -> Duration {
        Duration::from_secs_f32(self)
    }

    #[inline]
    fn minutes(self) -> Duration {
        Duration::from_secs_f32(self * 60.0)
    }

    #[inline]
    fn hours(self) -> Duration {
        Duration::from_secs_f32(self * 60.0 * 60.0)
    }
}
