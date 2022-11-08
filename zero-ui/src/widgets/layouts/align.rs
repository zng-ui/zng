use crate::prelude::new_widget::*;

#[widget($crate::widgets::layouts::align::center)]
mod center {
    use super::*;

    inherit!(widget_base::base);

    properties! {
        pub widget_base::child;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            if let Some(child) = wgt.capture_ui_node(property_id!(self::child)) {
                let child = align(child, Align::CENTER);
                wgt.set_child(child.boxed());
            }
        });
    }
}

/// Centralizes the content in the available space.
///
/// This is the equivalent of setting [`align`](fn@align) to [`Align::CENTER`], but as a widget.
pub fn center(child: impl UiNode) -> impl UiNode {
    center! { child; }
}
