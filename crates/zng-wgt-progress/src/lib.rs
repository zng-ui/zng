#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Progress indicator widget.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_wgt::prelude::*;
use zng_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};

pub use zng_task::Progress;

/// Progress indicator widget.
#[widget($crate::ProgressView {
    ($progress:expr) => {
        progress = $progress;
    };
})]
pub struct ProgressView(StyleMix<WidgetBase>);
impl ProgressView {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
        }
    }
}
impl_style_fn!(ProgressView);

context_var! {
    /// The progress status value in a [`ProgressView`](struct@ProgressView)
    pub static PROGRESS_VAR: Progress = Progress::indeterminate();
}

/// The progress status to be displayed.
///
/// This property sets the [`PROGRESS_VAR`].
#[property(CONTEXT, default(PROGRESS_VAR), widget_impl(ProgressView))]
fn progress(child: impl UiNode, progress: impl IntoVar<Progress>) -> impl UiNode {
    with_context_var(child, PROGRESS_VAR, progress)
}

/// Progress default style.
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
        }
    }
}
