//! App window and monitors manager.

mod control;
use control::*;

mod types;
pub use types::*;

mod monitor;
pub use monitor::*;

mod vars;
pub use vars::*;

mod service;
pub use service::*;

pub mod commands;

use crate::{
    app::{
        self,
        raw_events::{RawWindowFocusArgs, RAW_WINDOW_FOCUS_EVENT},
        AppExtended, AppExtension, ControlFlow, HeadlessApp,
    },
    context::{AppContext, WidgetUpdates, WindowContext},
    event::EventUpdate,
    image::ImageVar,
};

/// Application extension that manages windows.
///
/// # Events
///
/// Events this extension provides:
///
/// * [`WINDOW_OPEN_EVENT`]
/// * [`WINDOW_CHANGED_EVENT`]
/// * [`WINDOW_FOCUS_CHANGED_EVENT`]
/// * [`WINDOW_CLOSE_REQUESTED_EVENT`]
/// * [`WINDOW_CLOSE_EVENT`]
/// * [`MONITORS_CHANGED_EVENT`]
/// * [`WIDGET_INFO_CHANGED_EVENT`]
///
/// # Services
///
/// Services this extension provides:
///
/// * [`Windows`]
/// * [`Monitors`]
#[derive(Default)]
pub struct WindowManager {}
impl AppExtension for WindowManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Monitors::new());
        ctx.services.register(Windows::new(ctx.updates.sender()));
    }

    fn event_preview(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        Monitors::on_pre_event(ctx, update);
        Windows::on_pre_event(ctx, update);
    }

    fn event_ui(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        Windows::on_ui_event(ctx, update);
    }

    fn event(&mut self, ctx: &mut AppContext, update: &mut EventUpdate) {
        Windows::on_event(ctx, update);
    }

    fn update_ui(&mut self, ctx: &mut AppContext, updates: &mut WidgetUpdates) {
        Windows::on_ui_update(ctx, updates);
    }

    fn update(&mut self, ctx: &mut AppContext) {
        Windows::on_update(ctx);
    }

    fn layout(&mut self, ctx: &mut AppContext) {
        Windows::on_layout(ctx);
    }

    fn render(&mut self, ctx: &mut AppContext) {
        Windows::on_render(ctx);
    }
}

/// Extension trait, adds [`run_window`] to [`AppExtended`].
///
/// [`run_window`]: AppRunWindowExt::run_window
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// This method only returns when the app has exited.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::AppRunWindowExt;
    /// # macro_rules! window { ($($tt:tt)*) => { unimplemented!() } }
    /// App::default().run_window(|ctx| {
    ///     println!("starting app with window {:?}", ctx.window_id);
    ///     window! {
    ///         title = "Window 1";
    ///         child = text("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::Windows;
    /// # macro_rules! window { ($($tt:tt)*) => { unimplemented!() } }
    /// App::default().run(|ctx| {
    ///     Windows::req(ctx.services).open(|ctx| {
    ///         println!("starting app with window {:?}", ctx.window_id);
    ///         window! {
    ///             title = "Window 1";
    ///             child = text("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static);
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) {
        self.run(|ctx| {
            Windows::req(ctx).open(new_window);
        })
    }
}

/// Extension trait, adds window control methods to [`HeadlessApp`].
///
/// [`open_window`]: HeadlessAppWindowExt::open_window
/// [`HeadlessApp`]: app::HeadlessApp
pub trait HeadlessAppWindowExt {
    /// Open a new headless window and returns the new window ID.
    ///
    /// The `new_window` argument is the [`WindowContext`] of the new window.
    ///
    /// Returns the [`WindowId`] of the new window.
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId;

    /// Cause the headless window to think it is focused in the screen.
    fn focus_window(&mut self, window_id: WindowId);
    /// Cause the headless window to think focus moved away from it.
    fn blur_window(&mut self, window_id: WindowId);

    /// Copy the current frame pixels of the window.
    ///
    /// The var will update until it is loaded or error.
    fn window_frame_image(&mut self, window_id: WindowId) -> ImageVar;

    /// Sends a close request, returns if the window was found and closed.
    fn close_window(&mut self, window_id: WindowId) -> bool;

    /// Open a new headless window and update the app until the window closes.
    fn run_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static);
}
impl HeadlessAppWindowExt for HeadlessApp {
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) -> WindowId {
        let response = Windows::req(self).open(new_window);
        self.run_task(move |ctx| async move {
            response.wait_rsp(&ctx).await;
            response.rsp().unwrap().window_id
        })
        .unwrap()
    }

    fn focus_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(None, Some(window_id));
        RAW_WINDOW_FOCUS_EVENT.notify(self, args);
        let _ = self.update(false);
    }

    fn blur_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(Some(window_id), None);
        RAW_WINDOW_FOCUS_EVENT.notify(self, args);
        let _ = self.update(false);
    }

    fn window_frame_image(&mut self, window_id: WindowId) -> ImageVar {
        Windows::req(self).frame_image(window_id)
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        use app::raw_events::*;

        let args = RawWindowCloseRequestedArgs::now(window_id);
        RAW_WINDOW_CLOSE_REQUESTED_EVENT.notify(self, args);

        let mut requested = false;
        let mut closed = false;

        let _ = self.update_observe_event(
            |_, update| {
                if let Some(args) = WINDOW_CLOSE_REQUESTED_EVENT.on(update) {
                    requested |= args.windows.contains(&window_id);
                } else if let Some(args) = WINDOW_CLOSE_EVENT.on(update) {
                    closed |= args.windows.contains(&window_id);
                }
            },
            false,
        );

        assert_eq!(requested, closed);

        closed
    }

    fn run_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) {
        let window_id = self.open_window(new_window);
        while Windows::req(self).is_open(window_id) {
            if let ControlFlow::Exit = self.update(true) {
                return;
            }
        }
    }
}
