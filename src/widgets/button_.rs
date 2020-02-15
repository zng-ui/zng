use crate::widget;
use crate::widgets::container;

widget! {
    /// A clickable container.
    pub button: container;

    use crate::properties::on_click;

    default(self) {
        /// Button click event.
        on_click: required!;
    }
}

fn test(child: impl crate::core::UiNode) {
    use crate::properties::on_click;
    let btn = button! {
        on_click: |a|{};
        => child
    };
}
