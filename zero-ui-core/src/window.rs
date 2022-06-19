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

use crate::{
    app::{
        self,
        raw_events::{RawWindowFocusArgs, RawWindowFocusEvent},
        AppExtended, AppExtension, ControlFlow, HeadlessApp,
    },
    context::{AppContext, WindowContext},
    event::EventUpdateArgs,
    image::ImageVar,
    var::WithVars,
};

/// Application extension that manages windows.
///
/// # Events
///
/// Events this extension provides:
///
/// * [WindowOpenEvent]
/// * [WindowChangedEvent]
/// * [WindowFocusChangedEvent]
/// * [WindowCloseRequestedEvent]
/// * [WindowCloseEvent]
/// * [MonitorsChangedEvent]
/// * [WidgetInfoChangedEvent]
///
/// # Services
///
/// Services this extension provides:
///
/// * [Windows]
/// * [Monitors]
#[derive(Default)]
pub struct WindowManager {}
impl AppExtension for WindowManager {
    fn init(&mut self, ctx: &mut AppContext) {
        ctx.services.register(Monitors::new());
        ctx.services.register(Windows::new(ctx.updates.sender()));
    }

    fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        Monitors::on_pre_event(ctx, args);
        Windows::on_pre_event(ctx, args);
    }

    fn event_ui<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        Windows::on_ui_event(ctx, args);
    }

    fn event<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        Windows::on_event(ctx, args);
    }

    fn update_ui(&mut self, ctx: &mut AppContext) {
        Windows::on_ui_update(ctx);
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
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run_window(|ctx| {
    ///     println!("starting app with window {:?}", ctx.window_id);
    ///     window! {
    ///         title = "Window 1";
    ///         content = text("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::WindowsExt;
    /// # macro_rules! window { ($($tt:tt)*) => { todo!() } }
    /// App::default().run(|ctx| {
    ///     ctx.services.windows().open(|ctx| {
    ///         println!("starting app with window {:?}", ctx.window_id);
    ///         window! {
    ///             title = "Window 1";
    ///             content = text("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static);
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) {
        self.run(|ctx| {
            ctx.services.windows().open(new_window);
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
        let response = self.ctx().services.windows().open(new_window);
        self.run_task(move |ctx| async move {
            response.wait_rsp(&ctx).await;
            ctx.with_vars(|v| response.rsp(v).unwrap().window_id)
        })
        .unwrap()
    }

    fn focus_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(None, Some(window_id));
        RawWindowFocusEvent.notify(self.ctx().events, args);
        let _ = self.update(false);
    }

    fn blur_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(Some(window_id), None);
        RawWindowFocusEvent.notify(self.ctx().events, args);
        let _ = self.update(false);
    }

    fn window_frame_image(&mut self, window_id: WindowId) -> ImageVar {
        self.ctx().services.windows().frame_image(window_id)
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        use app::raw_events::*;

        let args = RawWindowCloseRequestedArgs::now(window_id);
        RawWindowCloseRequestedEvent.notify(self.ctx().events, args);

        let mut requested = false;
        let mut closed = false;

        let _ = self.update_observe_event(
            |_, args| {
                if let Some(args) = WindowCloseRequestedEvent.update(args) {
                    requested |= args.window_id == window_id;
                } else if let Some(args) = WindowCloseEvent.update(args) {
                    closed |= args.window_id == window_id;
                }
            },
            false,
        );

        assert_eq!(requested, closed);

        closed
    }

    fn run_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + 'static) {
        let window_id = self.open_window(new_window);
        while self.ctx().services.windows().is_open(window_id) {
            if let ControlFlow::Exit = self.update(true) {
                return;
            }
        }
    }
}
