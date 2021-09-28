//! Zero-Ui View Process API.
//!
//! Zero-Ui isolates all render and windowing related code to a different process (the view-process), this crate
//! provides the API that must be implemented to create a view-process backend, plus the [`Controller`] that
//! can be used from an app-process to spawn and communicate with a view-process.
//!
//! # VERSION
//!
//! The [`VERSION`] of the `zero-ui-vp-api` dependency must match in both *App-Process* and *View-Process*, otherwise a runtime
//! panic error is generated. Usually both processes are initialized from the same executable so this is not a problem.
//!
//! # `webrender_api`
//!
//! You must use the `webrender_api` that is re-exported as the [`webrender_api`] module. This is because Mozilla
//! does not follow the crate versioning and publishing conventions, so we depend on `webrender` as a git submodule.
//! The *version* re-exported is, usually, the latest commit that was included in the latest Firefox stable release and
//! breaking changes are tracked by the `zero-ui-vp-api` crate version.
//!

#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![cfg_attr(doc_nightly, feature(doc_cfg))]

use std::fmt;
use std::time::Duration;

use units::{DipPoint, DipSize, Px, PxPoint, PxRect};
#[doc(inline)]
pub use webrender_api;

use serde::{Deserialize, Serialize};

/// The *App Process* and *View Process* must be build using the same exact version and this is
/// validated during run-time, causing a panic if the versions don't match. Usually the same executable is used
/// for both processes so this is not a problem.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod units;

mod types;
pub use types::*;

mod ipc;
pub use ipc::*;

mod app_process;
pub use app_process::*;

mod view_process;
pub use view_process::*;

use webrender_api::{ColorF, DynamicProperties, Epoch, FontInstanceKey, FontKey, HitTestResult, IdNamespace, ImageKey, PipelineId};

/// Packaged API request.
#[derive(Serialize, Deserialize, Debug)]
pub struct Request(RequestData);
impl Request {
    /// Returns `true` if the request represents a new frame or frame update for the window.
    pub fn is_frame(&self, window_id: WinId) -> bool {
        matches!(&self.0, RequestData::render { id, .. } | RequestData::render_update { id, .. } if *id == window_id)
    }

    /// Returns `true` if the request is setting the position or size of the window.
    pub fn is_move_or_resize(&self, window_id: WinId) -> bool {
        matches!(
            &self.0,
            RequestData::set_position { id, .. }
            | RequestData::set_size { id, .. }
            | RequestData::set_max_size { id, .. }
            | RequestData::set_min_size { id, .. }
            if *id == window_id
        )
    }
}

/// Packaged API response.
#[derive(Serialize, Deserialize, Debug)]
pub struct Response(ResponseData);

macro_rules! TypeOrNil {
    ($T:ty) => {
        $T
    };
    () => {
        ()
    };
}

/// Declares the internal `Request` and `Response` enums, public methods in `Controller` and the public trait `ViewApp`, in the
/// controller it packs and sends the request and receives and unpacks the response. In the view it implements
/// the method.
macro_rules! declare_api {
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $method:ident(
                &mut $self:ident
                $(, $input:ident : $RequestType:ty)* $(,)?
            ) $(-> $ResponseType:ty)?;
        )*
    ) => {
        #[derive(Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        #[allow(clippy::large_enum_variant)]
        #[repr(u32)]
        enum RequestData {
            $(
                $(#[$meta])*
                $method { $($input: $RequestType),* },
            )*
        }
        impl fmt::Debug for RequestData {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                if f.alternate() {
                    match self {
                        $(
                            RequestData::$method { $($input),* } => write!(f, "{}{:#?}", stringify!($method), ($($input),*)),
                        )+
                    }
                } else {
                    match self {
                        $(
                            RequestData::$method { .. } => write!(f, "{}(..)", stringify!($method)),
                        )+
                    }
                }
            }
        }

        #[derive(Debug, Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        #[repr(u32)]
        enum ResponseData {
            $(
                $(#[$meta])*
                $method(TypeOrNil![$($ResponseType)?]),
            )*
        }

        #[allow(unused_parens)]
        impl Controller {
            $(
                $(#[$meta])*
                #[allow(clippy::too_many_arguments)]
                $vis fn $method(&mut self $(, $input: $RequestType)*) -> VpResult<TypeOrNil![$($ResponseType)?]> {
                    match self.talk(Request(RequestData::$method { $($input),* }))?.0 {
                        ResponseData::$method(r) => Ok(r),
                        _ => panic!("view-process did not respond correctly")
                    }
                }
            )*
        }

        /// The view-process API.
        pub trait Api {
            /// Already implemented, matches a request, calls the corresponding method and packages the response.
            fn respond(&mut self, request: Request) -> Response {
                match request.0 {
                    $(
                        #[allow(unused_doc_comments)]
                        $(#[$meta])* // for the cfg
                        RequestData::$method { $($input),* } => {
                            let r = self.$method($($input),*);
                            Response(ResponseData::$method(r))
                        }
                    )*
                }
            }

            $(
                $(#[$meta])*
                #[allow(clippy::too_many_arguments)]
                fn $method(&mut self, $($input: $RequestType),*) $(-> $ResponseType)?;
            )*
        }
    };
}
declare_api! {
    /// Returns the [`VERSION`].
    ///
    /// This method can be called before the [`startup`].
    ///
    /// [`startup`]: Api::startup
    fn api_version(&mut self) -> String;

    /// Called once on startup.
    ///
    /// Other methods are only called after this was called once.
    fn startup(&mut self, gen: ViewProcessGen, device_events: bool, headless: bool);

    /// Called once after shutdown, if running in a managed external process it will be killed after this call.
    fn exit(&mut self);

    /// Returns the primary monitor if there is any or the first available monitor or none if no monitor was found.
    pub fn primary_monitor(&mut self) -> Option<(MonId, MonitorInfo)>;

    /// Returns information about the specific monitor, if it exists.
    pub fn monitor_info(&mut self, id: MonId) -> Option<MonitorInfo>;

    /// Returns all available monitors.
    pub fn available_monitors(&mut self) -> Vec<(MonId, MonitorInfo)>;

    /// Open a window.
    ///
    /// Returns the window id, and renderer ids.
    pub fn open_window(&mut self, config: WindowConfig) -> (WinId, IdNamespace, PipelineId);

    /// Open a headless surface.
    ///
    /// This is a real renderer but not connected to any window, you can requests pixels to get the
    /// rendered frames.
    ///
    /// The surface is identified with a "window" id, but no window is created, also returns the renderer ids.
    pub fn open_headless(&mut self, config: HeadlessConfig) -> (WinId, IdNamespace, PipelineId);

    /// Close the window or headless surface.
    pub fn close_window(&mut self, id: WinId);

    /// Reads the system default text anti-aliasing config.
    pub fn text_aa(&mut self) -> TextAntiAliasing;

    /// Reads the system "double-click" config.
    pub fn multi_click_config(&mut self) -> MultiClickConfig;

    /// Returns `true` if animations are enabled in the operating system.
    ///
    /// People with photosensitive epilepsy usually disable animations system wide.
    pub fn animation_enabled(&mut self) -> bool;

    /// Retrieves the keyboard repeat-delay setting from the operating system.
    ///
    /// If the user holds a key pressed a new key-press event will happen every time this delay is elapsed.
    /// Note, depending on the hardware the real delay can be slightly different.
    ///
    /// There is no repeat flag in the `winit` key press event, so as a general rule we consider a second key-press
    /// without any other keyboard event within the window of time of twice this delay as a repeat.
    ///
    /// This delay can also be used as the text-boxes caret blink rate.
    pub fn key_repeat_delay(&mut self) -> Duration;

    /// Set window title.
    pub fn set_title(&mut self, id: WinId, title: String);

    /// Set window visible.
    pub fn set_visible(&mut self, id: WinId, visible: bool);

    /// Set if the window is "top-most".
    pub fn set_always_on_top(&mut self, id: WinId, always_on_top: bool);

    /// Set if the user can drag-move the window.
    pub fn set_movable(&mut self, id: WinId, movable: bool);

    /// Set if the user can resize the window.
    pub fn set_resizable(&mut self, id: WinId, resizable: bool);

    /// Set the window taskbar icon visibility.
    pub fn set_taskbar_visible(&mut self, id: WinId, visible: bool);

    /// Set the window parent and if `self` blocks the parent events while open (`modal`).
    pub fn set_parent(&mut self, id: WinId, parent: Option<WinId>, modal: bool);

    /// Set if the window is see-through.
    pub fn set_transparent(&mut self, id: WinId, transparent: bool);

    /// Set the window system border and title visibility.
    pub fn set_chrome_visible(&mut self, id: WinId, visible: bool);

    /// Set the window top-left offset, includes the window chrome (outer-position).
    pub fn set_position(&mut self, id: WinId, pos: DipPoint);

    /// Set the window content area size (inner-size).
    pub fn set_size(&mut self, id: WinId, size: DipSize, frame: FrameRequest);

    /// Set the window state.
    pub fn set_state(&mut self, id: WinId, state: WindowState);

    /// Set the headless surface are size (viewport size).
    pub fn set_headless_size(&mut self, id: WinId, size: DipSize, scale_factor: f32);

    /// Set the window minimum content area size.
    pub fn set_min_size(&mut self, id: WinId, size: DipSize);

    /// Set the window maximum content area size.
    pub fn set_max_size(&mut self, id: WinId, size: DipSize);

    /// Set the window icon.
    pub fn set_icon(&mut self, id: WinId, icon: Option<Icon>);

    /// Gets the root pipeline ID.
    pub fn pipeline_id(&mut self, id: WinId) -> PipelineId;

    /// Gets the resources namespace.
    pub fn namespace_id(&mut self, id: WinId) -> IdNamespace;

    /// Add an image resource to the window renderer.
    ///
    /// Returns the new image key.
    pub fn add_image(&mut self, id: WinId, descriptor: webrender_api::ImageDescriptor, data: ByteBuf) -> ImageKey;

    /// Replace the image resource in the window renderer.
    pub fn update_image(
        &mut self,
        id: WinId,
        key: ImageKey,
        descriptor: webrender_api::ImageDescriptor,
        data: ByteBuf,
    );

    /// Delete the image resource in the window renderer.
    pub fn delete_image(&mut self, id: WinId, key: ImageKey);

    /// Add a raw font resource to the window renderer.
    ///
    /// Returns the new font key.
    pub fn add_font(&mut self, id: WinId, bytes: ByteBuf, index: u32) -> FontKey;

    /// Delete the font resource in the window renderer.
    pub fn delete_font(&mut self, id: WinId, key: FontKey);

    /// Add a font instance to the window renderer.
    ///
    /// Returns the new instance key.
    pub fn add_font_instance(
        &mut self,
        id: WinId,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<webrender_api::FontInstanceOptions>,
        plataform_options: Option<webrender_api::FontInstancePlatformOptions>,
        variations: Vec<webrender_api::FontVariation>,
    ) -> FontInstanceKey;

    /// Delete a font instance.
    pub fn delete_font_instance(&mut self, id: WinId, instance_key: FontInstanceKey);

    /// Gets the window content area size.
    pub fn size(&mut self, id: WinId) -> DipSize;

    /// Gets the window scale factor.
    pub fn scale_factor(&mut self, id: WinId) -> f32;

    /// In Windows, set if the `Alt+F4` should not cause a window close request and instead generate a key-press event.
    pub fn set_allow_alt_f4(&mut self, id: WinId, allow: bool);

    /// Read all pixels of the current frame.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels(&mut self, id: WinId) -> FramePixels;

    /// `glReadPixels` a new buffer.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels_rect(&mut self, id: WinId, rect: PxRect) -> FramePixels;

    /// Get display items of the last rendered frame that intercept the `point`.
    ///
    /// Returns the frame ID and all hits from front-to-back.
    pub fn hit_test(&mut self, id: WinId, point: PxPoint) -> (Epoch, HitTestResult);

    /// Set the text anti-aliasing used in the window renderer.
    pub fn set_text_aa(&mut self, id: WinId, aa: TextAntiAliasing);

    /// Set the video mode used when the window is in exclusive fullscreen.
    pub fn set_video_mode(&mut self, id: WinId, mode: VideoMode);

    ///  Render a new frame.
    pub fn render(&mut self, id: WinId, frame: FrameRequest);

    /// Update the current frame and re-render it.
    pub fn render_update(&mut self, id: WinId, updates: DynamicProperties, clear_color: Option<ColorF>);

    /// Used for testing respawn.
    #[cfg(debug_assertions)]
    pub fn crash(&mut self);
}

pub(crate) type AnyResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
