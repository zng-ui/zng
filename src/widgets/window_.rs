use crate::core::{
    types::{rgb, WidgetId},
    window::Window,
};
#[doc(hidden)]
pub use crate::properties::{background_color, id, size, title};
use crate::widget;
use crate::widgets::container;

widget! {
    /// A window container.
    pub window: container;

    default {
        /// Window title.
        title: "";
        /// Window size. If set to a variable it is kept in sync.
        ///
        /// Does not include the OS window border.
        size: (800.0, 600.0);

        /// Window clear color.
        background_color: rgb(0.1, 0.1, 0.1);

        id: unset!;
        /// Unique identifier of the window root widget.
        root_id -> id: WidgetId::new_unique();
    }

    /// Manually initializes a new [`window`](super).
    #[inline]
    fn new(child, root_id, title, size, background_color) -> Window {
        Window::new(root_id.pop().0, title.pop().0, size.pop().0, background_color.pop().0, child)
    }
}
