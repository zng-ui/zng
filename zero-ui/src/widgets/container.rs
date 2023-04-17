use crate::prelude::new_widget::*;

/// Base single content container.
#[widget($crate::widgets::Container)]
pub struct Container(WidgetBase);
impl Container {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            if let Some(child) = wgt.capture_ui_node(property_id!(crate::core::widget_base::child)) {
                wgt.set_child(child);
            }
        });
    }

    widget_impl! {
        /// The content.
        ///
        /// Can be any type that implements [`UiNode`], any widget.
        ///
        /// [`UiNode`]: crate::core::widget_instance::UiNode
        pub crate::core::widget_base::child(child: impl UiNode);

        /// Spacing around content, inside the border.
        pub crate::properties::padding(padding: impl IntoVar<SideOffsets>);

        /// Content alignment.
        pub crate::properties::child_align(align: impl IntoVar<Align>);

        /// Content overflow clipping.
        pub crate::properties::clip_to_bounds(clip: impl IntoVar<bool>);
    }
}
