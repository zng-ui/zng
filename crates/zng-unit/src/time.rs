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
        Duration::from_secs_f32(self / 1000.0)
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
