//! Modal backdrop widget.

use zng_wgt::{prelude::*, *};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_input::gesture::{on_click, ClickArgs};
use zng_wgt_style::{impl_style_fn, style_fn, Style, StyleMix};

use crate::{Response, DIALOG};

/// Modal dialog parent widget that fills the window.
///
/// This widget is instantiated by [`DIALOG`] automatically, you can only customize it by setting the [`style_fn`](fn@style_fn)
/// contextual property. Note that the [`popup::close_delay`] and [`popup::is_close_delaying`] properties work with this widget.
///
/// [`popup::close_delay`]: fn@zng_wgt_layer::popup::close_delay
/// [`popup::is_close_delaying`]: fn@zng_wgt_layer::popup::is_close_delaying
#[widget($crate::backdrop::DialogBackdrop {
    ($child:expr) => {
        child = $child;
    }
})]
pub struct DialogBackdrop(StyleMix<Container>);
impl DialogBackdrop {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
            modal = true;
        }
    }
}
impl_style_fn!(DialogBackdrop);

/// Enables dialog close on click.
///
/// When enabled a click on the backdrop closes the dialog with [`Response::close`].
#[property(EVENT, default(false), widget_impl(DialogBackdrop))]
pub fn close_on_click(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    on_click(
        child,
        hn!(|args: &ClickArgs| {
            if enabled.get() {
                args.propagation().stop();
                DIALOG.respond(Response::close());
            }
        }),
    )
}

/// Dialog backdrop default style.
#[widget($crate::backdrop::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            #[easing(250.ms())]
            background_color = colors::BLACK.transparent();
            zng_wgt_layer::popup::close_delay = 250.ms();
            when *#is_inited && !*#zng_wgt_layer::popup::is_close_delaying {
                background_color = colors::BLACK.with_alpha(20.pct());
            }
        }
    }
}
