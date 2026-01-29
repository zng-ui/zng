//! Modal backdrop widget.

use zng_ext_window::WINDOWS;
use zng_wgt::{prelude::*, *};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_input::gesture::on_click;
use zng_wgt_layer::popup::{POPUP_CLOSE_REQUESTED_EVENT, PopupCloseRequestedArgs};
use zng_wgt_style::{Style, StyleMix, impl_style_fn};

use crate::DIALOG;

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

        // the backdrop widget id identifies the popup, so
        self.widget_builder()
            .push_build_action(|b| b.push_intrinsic(NestGroup::EVENT, "popup-pump", backdrop_node));

        widget_set! {
            self;
            modal = true;

            on_click = hn!(|args| {
                args.propagation.stop();
                DIALOG.respond_default();
            });
        }
    }
}
impl_style_fn!(DialogBackdrop, DefaultStyle);

/// Share popup events with the dialog child.
fn backdrop_node(child: impl IntoUiNode) -> UiNode {
    match_node(child, |_, op| {
        if let UiNodeOp::Init = op {
            let win_id = WINDOW.id();
            let id = WIDGET.id();
            WIDGET.push_var_handle(POPUP_CLOSE_REQUESTED_EVENT.hook(move |args| {
                if args.popup.widget_id() == id
                    && let Some(tree) = WINDOWS.widget_tree(win_id)
                    && let Some(info) = tree.get(id)
                {
                    for child in info.children() {
                        POPUP_CLOSE_REQUESTED_EVENT.notify(PopupCloseRequestedArgs::new(
                            args.timestamp,
                            args.propagation.clone(),
                            child.path(),
                        ));
                    }
                }
                true
            }));
            WIDGET.sub_event(&POPUP_CLOSE_REQUESTED_EVENT);
        }
    })
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
