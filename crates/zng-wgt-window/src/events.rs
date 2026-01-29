//! Window events, [`on_window_open`](fn@on_window_open) and [`on_window_close_requested`](fn@on_window_close_requested).
//!
//! These events are re-exported by [`Window!`](struct@crate::Window) as `on_open` and `on_close_requested`, but you can
//! attach then in any widget inside a window using the property full name.
//!
//! There is no event property for the [`WINDOW_CLOSE_EVENT`] because that event notifies
//! after the window is deinited. You can use [`on_deinit`](fn@zng_wgt::on_deinit) in the
//! [`Window!`](struct@crate::Window) widget to handle a window closing, or create an app level handler for the
//! event using [`EVENTS`](zng_app::event::EVENTS).
//!
//! [`WINDOW_CLOSE_EVENT`]: zng_ext_window::WINDOW_CLOSE_EVENT

use zng_ext_window::*;
use zng_wgt::prelude::*;

event_property! {
    /// On window opened.
    ///
    /// This event notifies once per window, after the window content is inited.
    #[property(EVENT)]
    pub fn on_window_open<on_pre_window_open>(child: impl IntoUiNode, handler: Handler<WindowOpenArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_OPEN_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id
            })
            .build::<PRE>(child, handler)
    }

    /// On window loaded.
    ///
    /// This event notifies once per window, after the first layout and all [`WindowLoadingHandle`]
    /// have expired or dropped.
    ///
    /// [`WindowLoadingHandle`]: zng_ext_window::WindowLoadingHandle
    #[property(EVENT)]
    pub fn on_window_load<on_pre_window_load>(child: impl IntoUiNode, handler: Handler<WindowOpenArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_LOAD_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id
            })
            .build::<PRE>(child, handler)
    }

    /// On window moved, resized or other state changed.
    ///
    /// This event aggregates events moves, resizes and other state changes into a
    /// single event to simplify tracking composite changes, for example, the window changes size and position
    /// when maximized, this can be trivially observed with this event.
    #[property(EVENT)]
    pub fn on_window_changed<on_pre_window_changed>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id
            })
            .build::<PRE>(child, handler)
    }

    /// On window position changed.
    #[property(EVENT)]
    pub fn on_window_moved<on_pre_window_moved>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.is_moved()
            })
            .build::<PRE>(child, handler)
    }

    /// On window size changed.
    #[property(EVENT)]
    pub fn on_window_resized<on_pre_window_resized>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.is_resized()
            })
            .build::<PRE>(child, handler)
    }

    /// On window close requested.
    ///
    /// Calling `propagation().stop()` on this event cancels the window close.
    #[property(EVENT)]
    pub fn on_window_close_requested<on_pre_window_close_requested>(
        child: impl IntoUiNode,
        handler: Handler<WindowCloseRequestedArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CLOSE_REQUESTED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.windows.contains(&id)
            })
            .build::<PRE>(child, handler)
    }

    /// On window closed.
    ///
    /// See [`WINDOW_CLOSE_EVENT`] for more details of when this event notifies.
    #[property(EVENT)]
    pub fn on_window_close<on_pre_window_close>(child: impl IntoUiNode, handler: Handler<WindowCloseArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CLOSE_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.windows.contains(&id)
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed.
    ///
    /// This event notifies every time the user or the app changes the [`WindowVars::state`].
    ///
    /// [`WindowVars::state`]: zng_ext_window::WindowVars
    #[property(EVENT)]
    pub fn on_window_state_changed<on_pre_window_state_changed>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.is_state_changed()
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed to [`WindowState::Maximized`].
    ///
    /// [`WindowState::Maximized`]: zng_ext_window::WindowState::Maximized
    #[property(EVENT)]
    pub fn on_window_maximized<on_pre_window_maximized>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.entered_state(WindowState::Maximized)
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed from [`WindowState::Maximized`].
    ///
    /// [`WindowState::Maximized`]: zng_ext_window::WindowState::Maximized
    #[property(EVENT)]
    pub fn on_window_unmaximized<on_pre_window_unmaximized>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.exited_state(WindowState::Maximized)
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed to [`WindowState::Minimized`].
    ///
    /// [`WindowState::Minimized`]: zng_ext_window::WindowState::Minimized
    #[property(EVENT)]
    pub fn on_window_minimized<on_pre_window_minimized>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.entered_state(WindowState::Minimized)
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed from [`WindowState::Minimized`].
    ///
    /// [`WindowState::Minimized`]: zng_ext_window::WindowState::Minimized
    #[property(EVENT)]
    pub fn on_window_unminimized<on_pre_window_unminimized>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.exited_state(WindowState::Minimized)
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed to [`WindowState::Normal`].
    ///
    /// [`WindowState::Normal`]: zng_ext_window::WindowState::Normal
    #[property(EVENT)]
    pub fn on_window_restored<on_pre_window_restored>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.entered_state(WindowState::Normal)
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed to [`WindowState::is_fullscreen`].
    ///
    /// [`WindowState::is_fullscreen`]: zng_ext_window::WindowState::is_fullscreen
    #[property(EVENT)]
    pub fn on_window_fullscreen<on_pre_window_fullscreen>(child: impl IntoUiNode, handler: Handler<WindowChangedArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.entered_fullscreen()
            })
            .build::<PRE>(child, handler)
    }

    /// On window state changed from [`WindowState::is_fullscreen`].
    ///
    /// [`WindowState::is_fullscreen`]: zng_ext_window::WindowState::is_fullscreen
    #[property(EVENT)]
    pub fn on_window_exited_fullscreen<on_pre_window_exited_fullscreen>(
        child: impl IntoUiNode,
        handler: Handler<WindowChangedArgs>,
    ) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(WINDOW_CHANGED_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id && args.exited_fullscreen()
            })
            .build::<PRE>(child, handler)
    }

    /// On Input Method Editor event.
    #[property(EVENT)]
    pub fn on_ime<on_pre_ime>(child: impl IntoUiNode, handler: Handler<ImeArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(IME_EVENT)
            .filter(|| {
                let id = WIDGET.id();
                move |args| args.target.widget_id() == id
            })
            .build::<PRE>(child, handler)
    }
}

#[cfg(feature = "image")]
event_property! {
    /// On window frame rendered.
    #[property(EVENT)]
    pub fn on_frame_image_ready<on_pre_frame_image_ready>(child: impl IntoUiNode, handler: Handler<FrameImageReadyArgs>) -> UiNode {
        const PRE: bool;
        EventNodeBuilder::new(FRAME_IMAGE_READY_EVENT)
            .filter(|| {
                let id = WINDOW.id();
                move |args| args.window_id == id
            })
            .build::<PRE>(child, handler)
    }
}
