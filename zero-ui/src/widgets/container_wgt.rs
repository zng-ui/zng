use crate::prelude::new_widget::*;

/// Base single content container.
#[widget($crate::widgets::Container)]
pub struct Container(widget_base::WidgetBase);
impl Container {
    #[widget(on_start)]
    fn on_start(&mut self) {
        self.builder().push_build_action(|wgt| {
            if let Some(child) = wgt.capture_ui_node(property_id!(child)) {
                wgt.set_child(child);
            }
        });
    }

    /// The content.
    ///
    /// Can be any type that implements [`UiNode`], any widget.
    ///
    /// [`UiNode`]: crate::core::widget_instance::UiNode
    #[property(crate::core::widget_base::child)]
    pub fn child(&self, child: impl UiNode) { }
}

/// Defines the container child.
#[property(CHILD, capture, default(FillUiNode), impl(Container))]
pub fn child(_child: impl UiNode, child: impl UiNode) -> impl UiNode {
    _child
}

mod old {
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
