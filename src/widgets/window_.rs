use crate::core::{types::rgb, window::Window};
#[doc(hidden)]
pub use crate::properties::{background_color, size, title};
use crate::widget;
use crate::widgets::container;

widget! {
    /// A window container.
    pub window: container;

    default(self) {
        /// Window title.
        title: "";
        /// Window size. If set to a variable it is kept in sync.
        ///
        /// Does not include the OS window border.
        size: (800.0, 600.0);

        /// Window clear color.
        background_color: rgb(1.0, 1.0, 1.0);
    }

    /// Manually initializes a new [`window`](super).
    #[inline]
    fn new(child, id, title, size, background_color) -> Window {
        Window::new(id.pop().0, title.pop().0, size.pop().0, background_color.pop().0, child)
    }
}
