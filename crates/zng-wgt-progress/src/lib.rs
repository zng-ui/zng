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
use zng_wgt_container::{child_out_bottom, Container};
use zng_wgt_fill::background_color;
use zng_wgt_size_offset::{height, width, x};
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
pub fn progress(child: impl UiNode, progress: impl IntoVar<Progress>) -> impl UiNode {
    with_context_var(child, PROGRESS_VAR, progress)
}

/// Progress view default style (progress bar with message text).
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            zng_wgt_fill::background = Container! {
                height = 8;
                clip_to_bounds = true;
                child_align = Align::FILL_START;
                child = zng_wgt::Wgt! {
                    background_color = colors::ACCENT_COLOR_VAR.rgba();

                    width = PROGRESS_VAR.map(|p| Length::from(p.fct()));
                    when *#{PROGRESS_VAR.map(|p| p.is_indeterminate())} {
                        width = 10.pct();
                        x = 10; // !!:TODO animate
                    }
                };
            };

            child_out_bottom = zng_wgt_text::Text! {
                txt = PROGRESS_VAR.map(|p| p.msg());
                zng_wgt::visibility = PROGRESS_VAR.map(|p| Visibility::from(!p.msg().is_empty()));
                zng_wgt::align = Align::CENTER;
            }, 4;
        }
    }
}
