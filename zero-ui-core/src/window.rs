//! App window and monitors manager.

mod control;
use control::*;

mod ime;
pub use ime::*;

mod types;
pub use types::*;

mod monitor;
pub use monitor::*;

mod vars;
pub use vars::*;

mod service;
pub use service::*;
use zero_ui_view_api::image::ImageMaskMode;

use std::future::Future;

pub mod commands;

use crate::{
    app::{
        self,
        raw_events::{RawWindowFocusArgs, RAW_WINDOW_FOCUS_EVENT},
        AppExtended, AppExtension, ControlFlow, HeadlessApp,
    },
    context::{InfoUpdates, LayoutUpdates, RenderUpdates, WidgetUpdates},
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
/// ///
/// # Default
///
/// This extension is included in the [default app].
///
/// [default app]: crate::app::App::default
#[derive(Default)]
pub struct WindowManager {}
impl AppExtension for WindowManager {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        MonitorsService::on_pre_event(update);
        WINDOWS::on_pre_event(update);
    }

    fn event_ui(&mut self, update: &mut EventUpdate) {
        WINDOWS::on_ui_event(update);
    }

    fn event(&mut self, update: &mut EventUpdate) {
        WINDOWS::on_event(update);
    }

    fn update_ui(&mut self, update_widgets: &mut WidgetUpdates) {
        WINDOWS::on_ui_update(update_widgets);
    }

    fn update(&mut self) {
        WINDOWS::on_update();
    }

    fn info(&mut self, info_widgets: &mut InfoUpdates) {
        WINDOWS::on_info(info_widgets);
    }

    fn layout(&mut self, layout_widgets: &mut LayoutUpdates) {
        WINDOWS::on_layout(layout_widgets);
    }

    fn render(&mut self, render_widgets: &mut RenderUpdates, render_update_widgets: &mut RenderUpdates) {
        WINDOWS::on_render(render_widgets, render_update_widgets);
    }
}

/// Extension trait, adds [`run_window`] to [`AppExtended`].
///
/// [`run_window`]: AppRunWindowExt::run_window
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// The `new_window` future runs inside the [`WINDOW`] context of the new window, the window opens after the future returns it.
    ///
    /// This method only returns when the app has exited.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::context::WINDOW;
    /// # use zero_ui_core::window::AppRunWindowExt;
    /// # macro_rules! Window { ($($tt:tt)*) => { unimplemented!() } }
    /// App::default().run_window(async {
    ///     println!("starting app with window {:?}", WINDOW.id());
    ///     Window! {
    ///         title = "Window 1";
    ///         child = Text!("Window 1");
    ///     }
    /// })   
    /// ```
    ///
    /// Which is a shortcut for:
    /// ```no_run
    /// # use zero_ui_core::app::App;
    /// # use zero_ui_core::context::WINDOW;
    /// # use zero_ui_core::window::WINDOWS;
    /// # macro_rules! Window { ($($tt:tt)*) => { unimplemented!() } }
    /// App::default().run(async {
    ///     WINDOWS.open(async {
    ///         println!("starting app with window {:?}", WINDOW.id());
    ///         Window! {
    ///             title = "Window 1";
    ///             child = Text!("Window 1");
    ///         }
    ///     });
    /// })   
    /// ```
    ///
    /// [`WINDOW`]: crate::context::WINDOW
    fn run_window<F>(self, new_window: F)
    where
        F: Future<Output = WindowRoot> + Send + 'static;
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window<F>(self, new_window: F)
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        self.run(async move {
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
    /// The `new_window` runs inside the [`WINDOW`] context of the new window.
    ///
    /// Returns the [`WindowId`] of the new window after the window is open and loaded and has generated one frame.
    ///
    /// [`WINDOW`]: crate::context::WINDOW
    fn open_window<F>(&mut self, new_window: F) -> WindowId
    where
        F: Future<Output = WindowRoot> + Send + 'static;

    /// Cause the headless window to think it is focused in the screen.
    fn focus_window(&mut self, window_id: WindowId);
    /// Cause the headless window to think focus moved away from it.
    fn blur_window(&mut self, window_id: WindowId);

    /// Copy the current frame pixels of the window.
    ///
    /// The var will update until it is loaded or error.
    fn window_frame_image(&mut self, window_id: WindowId, mask: Option<ImageMaskMode>) -> ImageVar;

    /// Sends a close request, returns if the window was found and closed.
    fn close_window(&mut self, window_id: WindowId) -> bool;

    /// Open a new headless window and update the app until the window closes.
    fn run_window<F>(&mut self, new_window: F)
    where
        F: Send + Future<Output = WindowRoot> + 'static;
}
impl HeadlessAppWindowExt for HeadlessApp {
    fn open_window<F>(&mut self, new_window: F) -> WindowId
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        app::App::extensions().require::<WindowManager>();

        let response = WINDOWS.open(new_window);
        self.run_task(async move {
            let window_id = response.wait_rsp().await;
            if !WINDOWS.is_loaded(window_id) {
                let rcv = FRAME_IMAGE_READY_EVENT.receiver();
                while let Ok(args) = rcv.recv_async().await {
                    if args.window_id == window_id {
                        break;
                    }
                }
            }
            window_id
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

    fn window_frame_image(&mut self, window_id: WindowId, mask: Option<ImageMaskMode>) -> ImageVar {
        WINDOWS.frame_image(window_id, mask)
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

    fn run_window<F>(&mut self, new_window: F)
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        let window_id = self.open_window(new_window);
        while WINDOWS.is_open(window_id) {
            if let ControlFlow::Exit = self.update(true) {
                return;
            }
        }
    }
}
