use crate::core::UiNode;
use crate::widget;

widget! {
    //! Custom container widget

    child_properties { }
    self_properties { }

    pub fn container(child: impl UiNode) -> impl UiNode {
        child
    }
}
