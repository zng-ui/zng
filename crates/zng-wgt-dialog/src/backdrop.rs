//! Modal backdrop widget.

use zng_wgt::{prelude::*, *};
use zng_wgt_container::Container;
use zng_wgt_fill::background_color;
use zng_wgt_input::gesture::on_click;
use zng_wgt_layer::popup::POPUP_CLOSE_REQUESTED_EVENT;
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};

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

        self.widget_builder()
            .push_build_action(|b| b.push_intrinsic(NestGroup::EVENT, "popup-pump", backdrop_node));

        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
            modal = true;

            on_click = hn!(|args| {
                args.propagation().stop();
                DIALOG.respond_default();
            });
        }
    }
}
impl_style_fn!(DialogBackdrop);

/// Share popup events with the dialog child.
fn backdrop_node(child: impl IntoUiNode) -> UiNode {
    match_node(child, |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&POPUP_CLOSE_REQUESTED_EVENT);
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = POPUP_CLOSE_REQUESTED_EVENT.on(update) {
                for child in WIDGET.info().descendants() {
                    if POPUP_CLOSE_REQUESTED_EVENT.is_subscriber(child.id()) {
                        let mut delivery = UpdateDeliveryList::new_any();
                        delivery.insert_wgt(&child);
                        let update = POPUP_CLOSE_REQUESTED_EVENT.new_update_custom(args.clone(), delivery);
                        c.event(&update);
                    }
                }
            }
        }
        _ => {}
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
