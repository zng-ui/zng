use zng_app::{AppBuilder, AppControlFlow, HeadlessApp, view_process::raw_events::{RAW_WINDOW_FOCUS_EVENT, RawWindowFocusArgs}, window::WindowId};

use crate::{CloseWindowResult, WINDOWS, WindowInstanceState, WindowRoot, WindowVars};

/// Extension trait, adds [`run_window`] to [`AppBuilder`].
///
/// [`run_window`]: AppRunWindowExt::run_window
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
impl AppRunWindowExt for AppBuilder {
    fn run_window<F>(self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        let new_window = new_window.into_future();
        self.run(async move {
            WINDOWS.open(WindowId::new_unique(), new_window);
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
    fn open_window<F>(&mut self, window_id: WindowId, new_window: impl IntoFuture<IntoFuture = F>) -> WindowVars
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
    fn run_window<F>(&mut self, window_id: WindowId, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Send + Future<Output = WindowRoot> + 'static;

    /// Open a new headless window and update the app until the window closes or 60 seconds elapse.
    #[cfg(any(test, doc, feature = "test_util"))]
    fn doc_test_window<F>(&mut self, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + 'static + Send;
}
impl HeadlessAppWindowExt for HeadlessApp {
    fn open_window<F>(&mut self, window_id: WindowId, new_window: impl IntoFuture<IntoFuture = F>) -> WindowVars
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        let response = WINDOWS.open(window_id, new_window);
        self.run_task(async move {
            let vars = response.wait_rsp().await;
            vars.instance_state()
                .wait_match(|s| !matches!(s, WindowInstanceState::Building))
                .await;
            vars
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
        let r = WINDOWS.close(window_id);
        let r = self.run_task(async move { r.wait_rsp().await });
        r.is_none() || matches!(r.unwrap(), CloseWindowResult::Closed)
    }

    fn run_window<F>(&mut self, window_id: WindowId, new_window: impl IntoFuture<IntoFuture = F>)
    where
        F: Future<Output = WindowRoot> + Send + 'static,
    {
        let state = self.open_window(window_id, new_window).instance_state();
        while !matches!(state.get(), WindowInstanceState::Closed) {
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
        let state = self.open_window(WindowId::new_unique(), new_window).instance_state();

        while !matches!(state.get(), WindowInstanceState::Closed) {
            if let AppControlFlow::Exit = self.update(true) {
                return;
            }
            if timer.get().has_elapsed() {
                panic!("doc_test_window reached 60s deadline");
            }
        }
    }
}
