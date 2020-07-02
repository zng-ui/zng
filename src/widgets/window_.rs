use crate::core::widget;
use crate::core::{
    types::{rgb, WidgetId},
    window::Window,
};
use crate::properties::{background_color, size, title};
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

    /// Manually initializes a new [`window`](self).
    #[inline]
    fn new(child, root_id, title, size, background_color) -> Window {
        Window::new(root_id.unwrap(), title.unwrap(), size.unwrap(), background_color.unwrap(), child)
    }
}
