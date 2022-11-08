use crate::prelude::new_widget::*;

/// Base single content container.
#[widget($crate::widgets::container)]
pub mod container {
    use super::*;

    inherit!(widget_base::base);

    properties! {
        /// The content.
        ///
        /// Can be any type that implements [`UiNode`], any widget.
        ///
        /// [`UiNode`]: crate::core::widget_instance::UiNode
        pub crate::core::widget_base::child;

        /// Spacing around content, inside the border.
        pub crate::properties::padding;

        /// Content alignment.
        pub crate::properties::child_align;

        /// Content overflow clipping.
        pub crate::properties::clip_to_bounds;
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            if let Some(child) = wgt.capture_ui_node(property_id!(self::child)) {
                wgt.set_child(child);
            }
        });
    }
}
