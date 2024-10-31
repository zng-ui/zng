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
    /// The `factor` is clamped to the `0..=1` range.
    pub fn from_factor(factor: impl Into<Factor>) -> Self {
        Self::new(factor.into().clamp(0.0, 1.0))
    }

    fn new(value: Factor) -> Self {
        Self {
            factor: value,
            message: Txt::from_static(""),
            meta: Arc::new(RwLock::new(OwnedStateMap::new())),
        }
    }

    /// New with completed `n` of `total`.
    pub fn from_n_of(n: usize, total: usize) -> Self {
        Self::from_factor(total as f32 / n as f32)
    }

    /// Set the display message about the task status update.
    pub fn with_message(mut self, msg: impl Into<Txt>) -> Self {
        self.message = msg.into();
        self
    }

    /// Set custom status metadata for writing.
    pub fn with_meta_mut(self, meta: impl FnOnce(StateMapMut<Progress>)) -> Self {
        meta(self.meta.write().borrow_mut());
        self
    }

    /// Combine the factor completed [`value`] with another `factor`.
    ///
    /// The `factor` is clamped to the `0..=1` range.
    ///
    /// [`value`]: Self::value
    pub fn and_factor(mut self, factor: impl Into<Factor>) -> Self {
        let factor = factor.into().clamp(0.0, 1.0);
        if self.is_indeterminate() {
            self.factor = factor;
        } else {
            self.factor = (self.factor + factor) / 2.fct();
        }
        self
    }

    /// Combine the factor completed [`value`] with another factor computed from `n` of `total`.
    ///
    /// [`value`]: Self::value
    pub fn and_n_of(self, n: usize, total: usize) -> Self {
        self.and_factor(n.min(total) as f32 / total as f32)
    }

    /// Factor completed.
    ///
    /// Is `-1.fct()` for indeterminate, otherwise is a value in the `0..=1` range, `1.fct()` indicates task completion.
    pub fn factor(&self) -> Factor {
        self.factor
    }

    /// Factor of completion cannot be known.
    pub fn is_indeterminate(&self) -> bool {
        self.factor < 0.fct()
    }

    /// Task has completed.
    pub fn is_completed(&self) -> bool {
        self.factor() >= 1.fct()
    }

    /// Display text about the task status update.
    pub fn message(&self) -> Txt {
        self.message.clone()
    }

    /// Borrow the custom status metadata for reading.
    pub fn with_meta<T>(&self, visitor: impl FnOnce(StateMapRef<Progress>) -> T) -> T {
        visitor(self.meta.read().borrow())
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
        Progress::from_factor(completed)
    }
    fn from(completed: FactorPercent) -> Progress {
        Progress::from_factor(completed)
    }
    fn from(completed: f32) -> Progress {
        Progress::from_factor(completed)
    }
    fn from(status: Progress) -> Factor {
        status.factor()
    }
    fn from(status: Progress) -> FactorPercent {
        status.factor().pct()
    }
    fn from(status: Progress) -> f32 {
        status.factor().0
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
            true => Progress::from_factor(true),
        }
    }
}
