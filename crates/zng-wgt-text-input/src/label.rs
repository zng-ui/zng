//! Label text.

use zng_app::access::ACCESS_CLICK_EVENT;
use zng_ext_input::{
    focus::{FOCUS, FocusInfoBuilder},
    mouse::MOUSE_INPUT_EVENT,
    touch::TOUCH_INPUT_EVENT,
};
use zng_wgt::prelude::*;
use zng_wgt_input::focus::FocusableMix;
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};

/// Styleable and focusable read-only text widget.
///
/// Optionally can be the label of a [`target`](#method.target) widget, in this case the label is not focusable, it transfers focus
/// to the target.
///
/// # Shorthand
///
/// The widget macro supports the shorthand that sets the `txt` and `target` properties.
///
/// ```
/// # zng_wgt::enable_widget_macros!();
/// # use zng_wgt::prelude::*;
/// # use zng_wgt_text_input::label::*;
/// #
/// # fn main() {
/// # let _scope = zng_app::APP.minimal();
/// let label = Label!("txt", "target");
/// # }
/// ```
#[widget($crate::label::Label {
    ($txt:expr, $target:expr $(,)?) => {
        txt = $txt;
        target = $target;
    };
})]
pub struct Label(FocusableMix<StyleMix<zng_wgt_text::Text>>);
impl Label {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;
            style_base_fn = style_fn!(|_| DefaultStyle!());
        }
    }
}
impl_style_fn!(Label);

/// Default label style.
#[widget($crate::label::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            replace = true;
        }
    }
}

/// Defines the widget the label is for.
///
/// When the label is pressed the widget or the first focusable child of the widget is focused.
/// Accessibility metadata is also set so the target widget is marked as *labelled-by* this widget in the view-process.
///
/// If this is set focusable is disabled on the label widget.
#[property(CONTEXT, widget_impl(Label))]
pub fn target(child: impl IntoUiNode, target: impl IntoVar<WidgetId>) -> UiNode {
    let target = target.into_var();
    let mut prev_target = None::<WidgetId>;

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&target)
                .sub_event(&MOUSE_INPUT_EVENT)
                .sub_event(&TOUCH_INPUT_EVENT)
                .sub_event(&ACCESS_CLICK_EVENT);
        }
        UiNodeOp::Info { info } => {
            c.info(info);

            FocusInfoBuilder::new(info).focusable(false);

            if let Some(mut a) = info.access() {
                let target = target.get();
                prev_target = Some(target);
                a.set_labels(target);
            }
        }
        UiNodeOp::Event { update } => {
            c.event(update);

            if let Some(args) = MOUSE_INPUT_EVENT.on(update) {
                if args.is_mouse_down() {
                    FOCUS.focus_widget_or_enter(target.get(), true, false);
                }
            } else if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
                if args.is_touch_start() {
                    FOCUS.focus_widget_or_enter(target.get(), true, false);
                }
            } else if let Some(args) = ACCESS_CLICK_EVENT.on(update)
                && args.is_primary
            {
                FOCUS.focus_widget_or_enter(target.get(), true, false);
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(id) = target.get_new() {
                if let Some(id) = prev_target.take() {
                    UPDATES.update_info(id);
                }
                UPDATES.update_info(id);
                WIDGET.update_info();
            }
        }
        _ => {}
    })
}
