use core::fmt;
use std::sync::Arc;

use parking_lot::RwLock;
use zng_state_map::{OwnedStateMap, StateMapMut, StateMapRef};
use zng_txt::Txt;
use zng_unit::{Factor, FactorPercent, FactorUnits as _};
use zng_var::impl_from_and_into_var;

/// Status update about a task progress.
#[derive(Clone)]
pub struct Progress {
    factor: Factor,
    message: Txt,
    meta: Arc<RwLock<OwnedStateMap<Progress>>>,
}
impl Progress {
    /// New indeterminate.
    pub fn indeterminate() -> Self {
        Self::new(-1.fct())
    }

    /// New completed.
    pub fn completed() -> Self {
        Self::new(1.fct())
    }

    /// New with a factor of completion.
    ///
    /// The `factor` must be in the `0..=1` range, with a rounding error of `0.001`, values outside this range
    /// are converted to indeterminate.
    pub fn from_fct(factor: impl Into<Factor>) -> Self {
        Self::new(factor.into())
    }

    /// New with completed `n` of `total`.
    pub fn from_n_of(n: usize, total: usize) -> Self {
        Self::new(Self::normalize_n_of(n, total))
    }

    /// Set the display message about the task status update.
    pub fn with_message(mut self, msg: impl Into<Txt>) -> Self {
        self.message = msg.into();
        self
    }

    /// Set custom status metadata for writing.
    ///
    /// Note that metadata is shared between all clones of `self`.
    pub fn with_meta_mut(self, meta: impl FnOnce(StateMapMut<Progress>)) -> Self {
        meta(self.meta.write().borrow_mut());
        self
    }

    /// Combine the factor completed [`fct`] with another `factor`.
    ///
    /// [`fct`]: Self::fct
    pub fn and_fct(mut self, factor: impl Into<Factor>) -> Self {
        if self.is_indeterminate() {
            return self;
        }
        let factor = Self::normalize_factor(factor.into());
        if factor < 0.fct() {
            // indeterminate
            self.factor = -1.fct();
        } else {
            self.factor = (self.factor + factor) / 2.fct();
        }
        self
    }

    /// Combine the factor completed [`fct`] with another factor computed from `n` of `total`.
    ///
    /// [`fct`]: Self::fct
    pub fn and_n_of(self, n: usize, total: usize) -> Self {
        self.and_fct(Self::normalize_n_of(n, total))
    }

    /// Replace the [`fct`] value with a new `factor`.
    ///
    /// [`fct`]: Self::fct
    pub fn with_fct(mut self, factor: impl Into<Factor>) -> Self {
        self.factor = Self::normalize_factor(factor.into());
        self
    }

    /// Replace the [`fct`] value with a new factor computed from `n` of `total`.
    ///
    /// [`fct`]: Self::fct
    pub fn with_n_of(mut self, n: usize, total: usize) -> Self {
        self.factor = Self::normalize_n_of(n, total);
        self
    }

    /// Factor completed.
    ///
    /// Is `-1.fct()` for indeterminate, otherwise is a value in the `0..=1` range, `1.fct()` indicates task completion.
    pub fn fct(&self) -> Factor {
        self.factor
    }

    /// Factor of completion cannot be known.
    pub fn is_indeterminate(&self) -> bool {
        self.factor < 0.fct()
    }

    /// Task has completed.
    pub fn is_completed(&self) -> bool {
        self.fct() >= 1.fct()
    }

    /// Display text about the task status update.
    pub fn message(&self) -> Txt {
        self.message.clone()
    }

    /// Borrow the custom status metadata for reading.
    pub fn with_meta<T>(&self, visitor: impl FnOnce(StateMapRef<Progress>) -> T) -> T {
        visitor(self.meta.read().borrow())
    }

    fn normalize_factor(mut value: Factor) -> Factor {
        if value.0 < 0.0 {
            if value.0 > -0.001 {
                value.0 = 0.0;
            } else {
                // too wrong, indeterminate
                value.0 = -1.0;
            }
        } else if value.0 > 1.0 {
            if value.0 < 1.001 {
                value.0 = 1.0;
            } else {
                value.0 = -1.0;
            }
        } else if !value.0.is_finite() {
            value.0 = -1.0;
        }
        value
    }

    fn normalize_n_of(n: usize, total: usize) -> Factor {
        if n > total {
            -1.fct() // invalid, indeterminate
        } else if total == 0 {
            1.fct() // 0 of 0, complete
        } else {
            Self::normalize_factor(Factor(n as f32 / total as f32))
        }
    }

    fn new(value: Factor) -> Self {
        Self {
            factor: Self::normalize_factor(value),
            message: Txt::from_static(""),
            meta: Arc::new(RwLock::new(OwnedStateMap::new())),
        }
    }
}
impl fmt::Debug for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskStatus")
            .field("factor", &self.factor)
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.message.is_empty() {
            write!(f, "{}", self.message)?;
            if !self.is_indeterminate() {
                write!(f, " ({})", self.factor.pct())
            } else {
                Ok(())
            }
        } else if !self.is_indeterminate() {
            write!(f, "{}", self.factor.pct())
        } else {
            Ok(())
        }
    }
}
impl PartialEq for Progress {
    fn eq(&self, other: &Self) -> bool {
        self.factor == other.factor && self.message == other.message && {
            let a = self.meta.read();
            let b = other.meta.read();
            let a = a.borrow();
            let b = b.borrow();
            a.is_empty() == b.is_empty() && (a.is_empty() || Arc::ptr_eq(&self.meta, &other.meta))
        }
    }
}
impl Eq for Progress {}
impl_from_and_into_var! {
    fn from(completed: Factor) -> Progress {
        Progress::from_fct(completed)
    }
    fn from(completed: FactorPercent) -> Progress {
        Progress::from_fct(completed)
    }
    fn from(completed: f32) -> Progress {
        Progress::from_fct(completed)
    }
    fn from(status: Progress) -> Factor {
        status.fct()
    }
    fn from(status: Progress) -> FactorPercent {
        status.fct().pct()
    }
    fn from(status: Progress) -> f32 {
        status.fct().0
    }
    fn from(n_total: (usize, usize)) -> Progress {
        Progress::from_n_of(n_total.0, n_total.1)
    }
    fn from(indeterminate_message: Txt) -> Progress {
        Progress::indeterminate().with_message(indeterminate_message)
    }
    fn from(indeterminate_message: &'static str) -> Progress {
        Progress::indeterminate().with_message(indeterminate_message)
    }
    fn from(indeterminate_or_completed: bool) -> Progress {
        match indeterminate_or_completed {
            false => Progress::indeterminate(),
            true => Progress::from_fct(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fct_n1() {
        let p = Progress::from_fct(-1.fct());
        assert_eq!(p, Progress::indeterminate());
    }

    #[test]
    fn fct_2() {
        let p = Progress::from_fct(2.fct());
        assert_eq!(p, Progress::indeterminate());
    }

    #[test]
    fn fct_05() {
        let p = Progress::from_fct(0.5.fct());
        assert_eq!(p, Progress::from(0.5.fct()));
    }

    #[test]
    fn fct_0() {
        let p = Progress::from_fct(0.fct());
        assert_eq!(p, Progress::from(0.fct()));
    }

    #[test]
    fn fct_1() {
        let p = Progress::from_fct(1.fct());
        assert_eq!(p, Progress::from(1.fct()));
    }

    #[test]
    fn zero_of_zero() {
        let p = Progress::from_n_of(0, 0);
        assert_eq!(p, Progress::completed());
    }

    #[test]
    fn ten_of_ten() {
        let p = Progress::from_n_of(10, 10);
        assert_eq!(p, Progress::completed());
    }

    #[test]
    fn ten_of_one() {
        let p = Progress::from_n_of(10, 1);
        assert_eq!(p, Progress::indeterminate());
    }

    #[test]
    fn five_of_ten() {
        let p = Progress::from_n_of(5, 10);
        assert_eq!(p, Progress::from(50.pct()));
    }
}
