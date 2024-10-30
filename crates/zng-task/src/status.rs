use core::fmt;
use std::sync::Arc;

use parking_lot::RwLock;
use zng_state_map::{OwnedStateMap, StateMapMut, StateMapRef};
use zng_txt::Txt;
use zng_unit::{Factor, FactorPercent};
use zng_var::impl_from_and_into_var;

/// Status update about a task progress.
#[derive(Clone)]
pub struct TaskStatus {
    /// Factor complete.
    ///
    /// If `None` the task completion is indeterminate.
    value: Option<Factor>,
    message: Txt,
    meta: Arc<RwLock<OwnedStateMap<TaskStatus>>>,
}
impl TaskStatus {
    /// New indeterminate.
    pub fn indeterminate() -> Self {
        Self {
            value: None,
            message: Txt::from_static(""),
            meta: Arc::new(RwLock::new(OwnedStateMap::new())),
        }
    }

    /// New with a factor of completion.
    pub fn completed(factor: impl Into<Factor>) -> Self {
        Self {
            value: Some(factor.into()),
            message: Txt::from_static(""),
            meta: Arc::new(RwLock::new(OwnedStateMap::new())),
        }
    }

    /// New with completed `n` of `total`.
    pub fn completed_of(n: usize, total: usize) -> Self {
        Self::completed(total as f32 / n as f32)
    }

    /// Set the display message about the task status update.
    pub fn with_message(mut self, msg: impl Into<Txt>) -> Self {
        self.message = msg.into();
        self
    }

    /// Set custom status metadata for writing.
    pub fn with_meta_mut(self, meta: impl FnOnce(StateMapMut<TaskStatus>)) -> Self {
        meta(self.meta.write().borrow_mut());
        self
    }

    /// Factor completed.
    ///
    /// Is `0.fct()` for indeterminate.
    pub fn value(&self) -> Factor {
        self.value.unwrap_or(Factor(0.0))
    }

    /// Factor of completion cannot be known.
    pub fn is_indeterminate(&self) -> bool {
        self.value.is_none()
    }

    /// Display text about the task status update.
    pub fn message(&self) -> Txt {
        self.message.clone()
    }

    /// Borrow the custom status metadata for reading.
    pub fn with_meta<T>(&self, visitor: impl FnOnce(StateMapRef<TaskStatus>) -> T) -> T {
        visitor(self.meta.read().borrow())
    }
}
impl fmt::Debug for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskStatus")
            .field("value", &self.value)
            .field("message", &self.message)
            .finish_non_exhaustive()
    }
}
impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.message.is_empty() {
            write!(f, "{}", self.message)?;
            if let Some(v) = self.value {
                write!(f, " ({})", v.pct())
            } else {
                Ok(())
            }
        } else if let Some(v) = self.value {
            write!(f, "{}", v.pct())
        } else {
            Ok(())
        }
    }
}
impl PartialEq for TaskStatus {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value && self.message == other.message && {
            let a = self.meta.read();
            let b = other.meta.read();
            let a = a.borrow();
            let b = b.borrow();
            a.is_empty() == b.is_empty() && (a.is_empty() || Arc::ptr_eq(&self.meta, &other.meta))
        }
    }
}
impl Eq for TaskStatus {}
impl_from_and_into_var! {
    fn from(completed: Factor) -> TaskStatus {
        TaskStatus::completed(completed)
    }
    fn from(completed: FactorPercent) -> TaskStatus {
        TaskStatus::completed(completed)
    }
    fn from(completed: f32) -> TaskStatus {
        TaskStatus::completed(completed)
    }
    fn from(status: TaskStatus) -> Factor {
        status.value()
    }
    fn from(status: TaskStatus) -> FactorPercent {
        status.value().pct()
    }
    fn from(status: TaskStatus) -> f32 {
        status.value().0
    }
    fn from(n_total: (usize, usize)) -> TaskStatus {
        TaskStatus::completed_of(n_total.0, n_total.1)
    }
    fn from(indeterminate_message: Txt) -> TaskStatus {
        TaskStatus::indeterminate().with_message(indeterminate_message)
    }
    fn from(indeterminate_message: &'static str) -> TaskStatus {
        TaskStatus::indeterminate().with_message(indeterminate_message)
    }
}
