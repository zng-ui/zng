use crate::core::widget;
use crate::core::{
    focus::{FocusScopeOnFocus, TabNav},
    keyboard::KeyInputArgs,
    types::{rgb, WidgetId},
    window::Window,
};
use crate::properties::{background_color, focus_scope, focus_scope_behavior, on_key_down, position, size, tab_nav, title, OnEventArgs};
use crate::widgets::container;
use zero_ui_macros::shortcut;

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
        ///  If set to a variable it is kept in sync.
        ///
        /// Set to [`f32::NAN`](f32::NAN) to not give an initial position.
        position: {
            // use shared var in debug to allow inspecting the value.
            #[cfg(debug_assertions)]
            let r = crate::core::var::var(crate::core::types::LayoutPoint::new(f32::NAN, f32::NAN));

            #[cfg(not(debug_assertions))]
            let r = (f32::NAN, f32::NAN);

            r
        };
        /// Window size.
        ///
        /// If set to a variable it is kept in sync.
        ///
        /// Does not include the OS window border.
        size: {
            #[cfg(debug_assertions)]
            let r = crate::core::var::var(crate::core::types::LayoutSize::new(800.0, 600.0));

            #[cfg(not(debug_assertions))]
            let r = (800.0, 600.0);

            r
        };

        /// Window clear color.
        background_color: rgb(0.1, 0.1, 0.1);

        id: unset!;
        /// Unique identifier of the window root widget.
        root_id -> id: WidgetId::new_unique();

        /// Windows are focus scopes by default.
        focus_scope: true;

        /// Windows cycle TAB navigation by default.
        tab_nav: TabNav::Cycle;

        /// Windows remember the last focused widget and return focus when activated again.
        focus_scope_behavior: FocusScopeOnFocus::LastFocused;

        /// Test inspector.
        on_key_down: print_frame_inspector();
    }

    /// Manually initializes a new [`window`](self).
    #[inline]
    fn new(child, root_id, title, position, size, background_color) -> Window {
        Window::new(root_id.unwrap(), title.unwrap(), position.unwrap(), size.unwrap(), background_color.unwrap(), child)
    }
}

#[cfg(not(debug_assertions))]
fn print_frame_inspector() -> impl FnMut(&mut OnEventArgs<KeyInputArgs>) {
    |_| {}
}

#[cfg(debug_assertions)]
fn print_frame_inspector() -> impl FnMut(&mut OnEventArgs<KeyInputArgs>) {
    use crate::core::debug::{write_frame, WriteFrameState};

    let mut state = WriteFrameState::none();
    move |args| {
        if args.args().shortcut() == Some(shortcut!(CTRL | SHIFT + I)) {
            let ctx = args.ctx();

            let frame = ctx
                .services
                .req::<crate::core::window::Windows>()
                .window(ctx.window_id)
                .unwrap()
                .frame_info();

            write_frame(frame, &state, &mut std::io::stderr());

            state = WriteFrameState::new(&frame);
        }
    }
}
