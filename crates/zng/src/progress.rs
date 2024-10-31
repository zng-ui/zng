//! Progress indicator widget, styles and properties.
//!
//! This widget displays [`task::Progress`] values that track the status of a running task.
//!
//! ```
//! use zng::prelude::*;
//!
//! let p = var(task::Progress::indeterminate());
//!
//! // on the view
//! let view = zng::progress::ProgressView!(p.clone());
//!
//! // on the controller/view-model
//! task::spawn(move async {
//!     for n in 0..=10 {
//!         task::deadline(500.ms()).await;
//!         p.set(task::Progress::from_n_of(n, 10).with_message(formatx!("sleeping {n} of 10")));
//!     }
//!     p.set(task::Progress:completed().with_message("done sleeping"));
//! });
//! ```
//!
//! [`task::Progress`]: zng::task::Progress
//!
//! # Full API
//!
//! See [`zng_wgt_progress`] and [`zng_task::Progress`] for the full widget API.

pub use zng_wgt_progress::{DefaultStyle, ProgressView, PROGRESS_VAR};
