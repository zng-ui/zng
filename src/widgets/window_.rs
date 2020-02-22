use crate::core::types::rgb;
#[doc(hidden)]
pub use crate::properties::{background_color, size};
use crate::widget;
use crate::widgets::container;

widget! {
    pub window: container;

    default(self) {
    //  title: "";
        size: (800.0,600.0);
        background_color: rgb(1.0, 1.0, 1.0);
    }

    //fn new(
    //    child: impl UiNode,
    //    id: WidgetId,
    //    title: impl IntoVar<Text>,
    //    size: impl IntoVar<LayoutSize>,
    //    background_color: impl IntoVar<ColorF>
    //) -> Window {
    //    Window::new(id, title, size, background_color, child)
    //}
}
