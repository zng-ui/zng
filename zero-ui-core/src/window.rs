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
    context::{WidgetUpdates, WindowContext},
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
/// * [`TRANSFORM_CHANGED_EVENT`]
/// * [`INTERACTIVITY_CHANGED_EVENT`]
///
/// # Services
///
/// Services this extension provides:
///
/// * [`WINDOWS`]
/// * [`MONITORS`]
#[derive(Default)]
pub struct WindowManager {}
impl AppExtension for WindowManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        MonitorsService::on_pre_event(update);
        WINDOWS::on_pre_event(update);
    }

    fn event_ui(&mut self, update: &mut EventUpdate) {
        WINDOWS::on_ui_event( update);
    }

    fn event(&mut self, update: &mut EventUpdate) {
        WINDOWS::on_event(update);
    }

    fn update_ui(&mut self,updates: &mut WidgetUpdates) {
        WINDOWS::on_ui_update(updates);
    }

    fn update(&mut self) {
        WINDOWS::on_update();
    }

    fn layout(&mut self) {
        WINDOWS::on_layout();
    }

    fn render(&mut self) {
        WINDOWS::on_render();
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
    ///         child = text!("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::window::WINDOWS;
    /// # macro_rules! window { ($($tt:tt)*) => { unimplemented!() } }
    /// App::default().run(|_| {
    ///     WINDOWS.open(|ctx| {
    ///         println!("starting app with window {:?}", ctx.window_id);
    ///         window! {
    ///             title = "Window 1";
    ///             child = text!("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + Send + 'static);
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window(self, new_window: impl FnOnce(&mut WindowContext) -> Window + Send + 'static) {
        self.run(|| {
            WINDOWS.open(new_window);
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
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + Send + 'static) -> WindowId;

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
    fn run_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + Send + 'static);
}
impl HeadlessAppWindowExt for HeadlessApp {
    fn open_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + Send + 'static) -> WindowId {
        let response = WINDOWS.open(new_window);
        self.run_task(move || async move {
            response.wait_rsp().await;
            response.rsp().unwrap().window_id
        })
        .unwrap()
    }

    fn focus_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(None, Some(window_id));
        RAW_WINDOW_FOCUS_EVENT.notify(args);
        let _ = self.update(false);
    }

    fn blur_window(&mut self, window_id: WindowId) {
        let args = RawWindowFocusArgs::now(Some(window_id), None);
        RAW_WINDOW_FOCUS_EVENT.notify(args);
        let _ = self.update(false);
    }

    fn window_frame_image(&mut self, window_id: WindowId) -> ImageVar {
        WINDOWS.frame_image(window_id)
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        use app::raw_events::*;

        let args = RawWindowCloseRequestedArgs::now(window_id);
        RAW_WINDOW_CLOSE_REQUESTED_EVENT.notify(args);

        let mut requested = false;
        let mut closed = false;

        let _ = self.update_observe_event(
            |update| {
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

    fn run_window(&mut self, new_window: impl FnOnce(&mut WindowContext) -> Window + Send + 'static) {
        let window_id = self.open_window(new_window);
        while WINDOWS.is_open(window_id) {
            if let ControlFlow::Exit = self.update(true) {
                return;
            }
        }
    }
}
