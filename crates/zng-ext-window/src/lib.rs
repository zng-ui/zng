#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! App window and monitors manager.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]
// suppress nag about very simple boxed closure signatures.
#![expect(clippy::type_complexity)]

#[macro_use]
extern crate bitflags;

mod control;
pub use control::{NestedWindowNode, NestedWindowWidgetInfoExt, OpenNestedHandlerArgs};

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

use zng_app::{
    AppControlFlow, AppExtended, AppExtension, HeadlessApp,
    update::{EventUpdate, InfoUpdates, LayoutUpdates, RenderUpdates, WidgetUpdates},
    view_process::raw_events::{RAW_WINDOW_FOCUS_EVENT, RawWindowFocusArgs},
    window::WindowId,
};
pub mod cmd;

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
///
/// # Services
///
/// Services this extension provides:
///
/// * [`WINDOWS`]
/// * [`MONITORS`]
///
/// The [`WINDOWS`] service is also setup as the implementer for [`IMAGES`] rendering.
///
/// [`IMAGES`]: zng_ext_image::IMAGES
#[derive(Default)]
#[non_exhaustive]
pub struct WindowManager {}
impl AppExtension for WindowManager {
    fn init(&mut self) {
        #[cfg(feature = "image")]
        zng_ext_image::IMAGES_WINDOW.hook_render_windows_service(Box::new(WINDOWS));
    }

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
/// [`AppExtended`]: zng_app::AppExtended
pub trait AppRunWindowExt {
    /// Runs the application event loop and requests a new window.
    ///
    /// The window opens after the future returns it. The [`WINDOW`] context for the new window is already available in the `new_window` future.
    ///
    /// This method only returns when the app has exited.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use zng_app::window::WINDOW;
    /// # use zng_app::APP;
    /// # use zng_ext_window::AppRunWindowExt as _;
    /// # trait AppDefaults { fn defaults(&self) -> zng_app::AppExtended<impl zng_app::AppExtension> { APP.minimal() } }
    /// # impl AppDefaults for APP { }
    /// # macro_rules! Window { ($($tt:tt)*) => { unimplemented!() } }
    /// APP.defaults().run_window(async {
    ///     println!("starting app with window {:?}", WINDOW.id());
    ///     Window! {
    ///         title = "Window 1";
    ///         child = Text!("Window 1");
    ///     }
    /// })
    /// ```
    ///
    /// Which is a shortcut for:
    ///
    /// ```no_run
    /// # use zng_app::window::WINDOW;
    /// # use zng_ext_window::WINDOWS;
    /// # use zng_app::APP;
    /// # use zng_ext_window::AppRunWindowExt as _;
    /// # trait AppDefaults { fn defaults(&self) -> zng_app::AppExtended<impl zng_app::AppExtension> { APP.minimal() } }
    /// # impl AppDefaults for APP { }
    /// # macro_rules! Window { ($($tt:tt)*) => { unimplemented!() } }
    /// APP.defaults().run(async {
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
    /// [`WINDOW`]: zng_app::window::WINDOW
    fn run_window<F>(self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + Send + 'static;
}
impl<E: AppExtension> AppRunWindowExt for AppExtended<E> {
    fn run_window<F>(self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        let new_window = new_window.into_future();
        self.run(async move {
            WINDOWS.open(new_window);
        })
    }
}

/// Window extension methods for [`HeadlessApp`].
///
/// [`open_window`]: HeadlessAppWindowExt::open_window
/// [`HeadlessApp`]: zng_app::HeadlessApp
pub trait HeadlessAppWindowExt {
    /// Open a new headless window and returns the new window ID.
    ///
    /// The `new_window` runs inside the [`WINDOW`] context of the new window.
    ///
    /// Returns the [`WindowId`] of the new window after the window is open and loaded and has generated one frame
    /// or if the window already closed before the first frame.
    ///
    /// [`WINDOW`]: zng_app::window::WINDOW
    /// [`WindowId`]: zng_app::window::WindowId
    fn open_window<F>(&mut self, new_window: impl IntoFuture<IntoFuture = F>) -> WindowId
    where
        F: Future<Output = WindowRoot> + Send + 'static;

    /// Cause the headless window to think it is focused in the screen.
    fn focus_window(&mut self, window_id: WindowId);
    /// Cause the headless window to think focus moved away from it.
    fn blur_window(&mut self, window_id: WindowId);

    /// Copy the current frame pixels of the window.
    ///
    /// The var will update until the image is loaded or error.
    #[cfg(feature = "image")]
    fn window_frame_image(&mut self, window_id: WindowId, mask: Option<zng_view_api::image::ImageMaskMode>) -> zng_ext_image::ImageVar;

    /// Sends a close request.
    ///
    /// Returns if the window was found and closed.
    fn close_window(&mut self, window_id: WindowId) -> bool;

    /// Open a new headless window and update the app until the window closes.
    fn run_window<F>(&mut self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Send + Future<Output = WindowRoot> + 'static;

    /// Open a new headless window and update the app until the window closes or 60 seconds elapse.
    #[cfg(any(test, doc, feature = "test_util"))]
    fn doc_test_window<F>(&mut self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + 'static + Send;
}
impl HeadlessAppWindowExt for HeadlessApp {
    fn open_window<F>(&mut self, new_window: impl IntoFuture<IntoFuture = F>) -> WindowId
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        zng_app::APP.extensions().require::<WindowManager>();

        let response = WINDOWS.open(new_window);
        self.run_task(async move {
            let window_id = response.wait_rsp().await;
            if !WINDOWS.is_loaded(window_id) {
                let close_rcv = WINDOW_CLOSE_EVENT.receiver();
                #[cfg(feature = "image")]
                let frame_rcv = FRAME_IMAGE_READY_EVENT.receiver();
                zng_task::any!(
                    async {
                        while let Ok(args) = close_rcv.recv_async().await {
                            if args.windows.contains(&window_id) {
                                break;
                            }
                        }
                    },
                    async {
                        #[cfg(feature = "image")]
                        while let Ok(args) = frame_rcv.recv_async().await {
                            if args.window_id == window_id {
                                break;
                            }
                        }
                    }
                )
                .await;
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

    #[cfg(feature = "image")]
    fn window_frame_image(&mut self, window_id: WindowId, mask: Option<zng_view_api::image::ImageMaskMode>) -> zng_ext_image::ImageVar {
        WINDOWS.frame_image(window_id, mask)
    }

    fn close_window(&mut self, window_id: WindowId) -> bool {
        use zng_app::view_process::raw_events::*;

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

    fn run_window<F>(&mut self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        let window_id = self.open_window(new_window);
        while WINDOWS.is_open(window_id) {
            if let AppControlFlow::Exit = self.update(true) {
                return;
            }
        }
    }

    #[cfg(any(test, doc, feature = "test_util"))]
    fn doc_test_window<F>(&mut self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        use zng_layout::unit::TimeUnits;

        let timer = zng_app::timer::TIMERS.deadline(60.secs());

        zng_task::spawn(async {
            zng_task::deadline(65.secs()).await;
            eprintln!("doc_test_window reached 65s fallback deadline");
            zng_env::exit(-1);
        });
        let window_id = self.open_window(new_window);

        while WINDOWS.is_open(window_id) {
            if let AppControlFlow::Exit = self.update(true) {
                return;
            }
            if timer.get().has_elapsed() {
                panic!("doc_test_window reached 60s deadline");
            }
        }
    }
}
