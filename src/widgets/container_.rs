use crate::widget;
use crate::core::UiNode;

widget! {
    //! Custom container widget

    child_properties { }
    self_properties { }

    pub fn container(child: impl UiNode) -> impl UiNode {
        child
    }
}