//! Label text.

use crate::core::focus::{FOCUS, FOCUS_CHANGED_EVENT};

use crate::prelude::new_widget::*;

/// Styleable and focusable read-only text widget.
///
/// Optionally can be the label of a [`target`] widget, automatically transferring focus to it.
#[widget($crate::widgets::Label)]
pub struct Label(FocusableMix<StyleMix<EnabledMix<text::Text>>>);
impl Label {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// Label style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
}

/// Default label style.
#[widget($crate::widgets::label::DefaultStyle)]
pub struct DefaultStyle(Style);

/// Defines the widget the label is for.
///
/// When the label is focused the widget or the first focusable child of the widget is focused.
/// Access metadata is also set so the target widget is marked as *labelled-by* this widget in the view-process.
#[property(CONTEXT, widget_impl(Label))]
pub fn target(child: impl UiNode, target: impl IntoVar<WidgetId>) -> impl UiNode {
    let target = target.into_var();
    let mut prev_target = None::<WidgetId>;

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&target).sub_event(&FOCUS_CHANGED_EVENT);
        }
        UiNodeOp::Info { info } => {
            if let Some(mut a) = info.access() {
                let target = target.get();
                prev_target = Some(target);
                a.set_labels(target);
            }
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            if let Some(args) = FOCUS_CHANGED_EVENT.on_unhandled(update) {
                if args.is_focus_enter(WIDGET.id()) {
                    args.propagation().stop();
                    FOCUS.focus_widget_or_enter(target.get(), true, args.highlight);
                }
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
