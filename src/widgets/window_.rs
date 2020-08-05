use crate::core::widget;
use crate::core::{
    focus::TabNav,
    types::{rgb, WidgetId},
    window::Window,
};
use crate::properties::{background_color, focus_scope, position, size, tab_nav, title};
use crate::widgets::container;

widget! {
    /// A window container.
    ///
    /// The instance type is [`Window`], witch can be given to the [`Windows`](crate::core::window::Windows) service
    /// to open a system window that is kept in sync with the window properties set in the widget.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use zero_ui::prelude::*;
    ///
    /// App::default().run_window(|_| {
    ///     window! {
    ///         title: "Window 1";
    ///         content: text("Window 1");
    ///     }
    /// })
    /// ```
    /// See [`run_window`](crate::core::window::AppRunWindow::run_window) for more details.
    pub window: container;

    default {
        /// Window title.
        title: "";

        /// Window position (left, top).
        ///
        /// Set to [`f32::NAN`](f32::NAN) to not give an initial position.
        position: (f32::NAN, f32::NAN);
        /// Window size. If set to a variable it is kept in sync.
        ///
        /// Does not include the OS window border.
        size: (800.0, 600.0);

        /// Window clear color.
        background_color: rgb(0.1, 0.1, 0.1);

        id: unset!;
        /// Unique identifier of the window root widget.
        root_id -> id: WidgetId::new_unique();

        /// Windows are focus scopes by default.
        focus_scope: true;

        /// Windows cycle TAB navigation by default.
        tab_nav: TabNav::Cycle;
    }

    /// Manually initializes a new [`window`](self).
    #[inline]
    fn new(child, root_id, title, position, size, background_color) -> Window {
        Window::new(root_id.unwrap(), title.unwrap(), position.unwrap(), size.unwrap(), background_color.unwrap(), child)
    }
}
