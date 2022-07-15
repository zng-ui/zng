//! Zero-Ui View Process API.
//!
//! Zero-Ui isolates all render and windowing related code to a different process (the view-process), this crate
//! provides the API that must be implemented to create a view-process backend, plus the [`Controller`] that
//! can be used from an app-process to spawn and communicate with a view-process.
//!
//! # VERSION
//!
//! The [`VERSION`] of this crate must match exactly in both *App-Process* and *View-Process*, otherwise a runtime
//! panic error is generated.
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

use units::{DipSize, Px, PxRect};
#[doc(inline)]
pub use webrender_api;

#[cfg(feature = "ipc")]
use serde::{Deserialize, Serialize};

/// The *App Process* and *View Process* must be build using the same exact version and this is
/// validated during run-time, causing a panic if the versions don't match.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod units;

mod types;
pub use types::*;

mod display_list;
pub use display_list::*;

mod ipc;
pub use ipc::*;

mod app_process;
pub use app_process::*;

mod view_process;
pub use view_process::*;

use webrender_api::{DocumentId, FontInstanceKey, FontKey, ImageKey};

/// Packaged API request.
#[derive(Debug)]
#[cfg_attr(feature = "ipc", derive(Serialize, Deserialize))]
pub struct Request(RequestData);
impl Request {
    /// Returns `true` if the request can only be made after the *init* event.
    pub fn must_be_online(&self) -> bool {
        !matches!(&self.0, RequestData::init { .. })
    }

    /// Returns `true` if the request represents a new frame or frame update for the window with the same wait ID.
    pub fn is_frame(&self, window_id: WindowId, wait_id: Option<FrameWaitId>) -> bool {
        match &self.0 {
            RequestData::render { id, frame } if *id == window_id && frame.wait_id == wait_id => true,
            RequestData::render_update { id, frame } if *id == window_id && frame.wait_id == wait_id => true,
            _ => false,
        }
    }

    /// Returns `true` if the request affects position or size of the window.
    pub fn affects_window_rect(&self, window_id: WindowId) -> bool {
        matches!(
            &self.0,
            RequestData::set_state { id, .. }
            if *id == window_id
        )
    }

    /// Returns `true` if this request will receive a response. Only [`Api`] methods
    /// that have a return value send back a response.
    pub fn expect_response(&self) -> bool {
        self.0.expect_response()
    }
}

/// Packaged API response.
#[derive(Debug)]
#[cfg_attr(feature = "ipc", derive(Serialize, Deserialize))]
pub struct Response(ResponseData);
impl Response {
    /// If this response must be send back to the app process. Only [`Api`] methods
    /// that have a return value send back a response.
    pub fn must_be_send(&self) -> bool {
        self.0.must_be_send()
    }
}

macro_rules! TypeOrNil {
    ($T:ty) => {
        $T
    };
    () => {
        ()
    };
}

macro_rules! type_is_some {
    (if $T:ty { $($t_true:tt)* } else { $($t_false:tt)* }) => {
        $($t_true)*
    };
    (if { $($t_true:tt)* } else { $($t_false:tt)* }) => {
        $($t_false)*
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
        #[cfg_attr(feature="ipc", derive(Serialize, Deserialize))]
        #[allow(non_camel_case_types)]
        #[allow(clippy::large_enum_variant)]
        #[repr(u32)]
        enum RequestData {
            $(
                $(#[$meta])*
                $method { $($input: $RequestType),* },
            )*
        }
        impl RequestData {
            #[allow(unused_doc_comments)]
            pub fn expect_response(&self) -> bool {
                match self {
                    $(
                        $(#[$meta])*
                        Self::$method { .. } => type_is_some! {
                            if $($ResponseType)? {
                                true
                            } else {
                                false
                            }
                        },
                    )*
                }
            }
        }
        impl fmt::Debug for RequestData {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                #[allow(unused_doc_comments)]
                if f.alternate() {
                    match self {
                        $(
                            $(#[$meta])*
                            RequestData::$method { $($input),* } => write!(f, "{}{:#?}", stringify!($method), ($($input),*)),
                        )+
                    }
                } else {
                    match self {
                        $(
                            $(#[$meta])*
                            RequestData::$method { .. } => write!(f, "{}(..)", stringify!($method)),
                        )+
                    }
                }
            }
        }

        #[derive(Debug)]
        #[cfg_attr(feature="ipc", derive(Serialize, Deserialize))]
        #[allow(non_camel_case_types)]
        #[repr(u32)]
        enum ResponseData {
            $(
                $(#[$meta])*
                $method(TypeOrNil![$($ResponseType)?]),
            )*
        }
        impl ResponseData {
            #[allow(unused_doc_comments)]
            pub fn must_be_send(&self) -> bool {
                match self {
                    $(
                        $(#[$meta])*
                        Self::$method(_) => type_is_some! {
                            if $($ResponseType)? {
                                true
                            } else {
                                false
                            }
                        },
                    )*
                }
            }
        }

        #[allow(unused_parens)]
        impl Controller {
            $(
                $(#[$meta])*
                #[allow(clippy::too_many_arguments)]
                $vis fn $method(&mut self $(, $input: $RequestType)*) -> VpResult<TypeOrNil![$($ResponseType)?]> {
                    let req = Request(RequestData::$method { $($input),* });
                    type_is_some! {
                        if $($ResponseType)? {
                            match self.talk(req)?.0 {
                                ResponseData::$method(r) => Ok(r),
                                r => panic!("view-process did not respond correctly for `{}`, {r:?}", stringify!($method))
                            }
                        } else {
                            self.command(req)
                        }
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
    /// Called once on init.
    ///
    /// Sends an [`Event::Inited`] once the view is completely online.
    /// Other methods may only be called after this event.
    fn init(&mut self, gen: ViewProcessGen, is_respawn: bool, device_events: bool, headless: bool);

    /// Called once after exit, if running in a managed external process it will be killed after this call.
    fn exit(&mut self);

    /// Open a window.
    ///
    /// Sends an [`Event::WindowOpened`] once the window, context and renderer have finished initializing or a
    /// [`Event::WindowOrHeadlessOpenError`] if it failed.
    pub fn open_window(&mut self, request: WindowRequest);

    /// Open a headless surface.
    ///
    /// This is a real renderer but not connected to any window, you can requests pixels to get the
    /// rendered frames.
    ///
    /// Sends an [`Event::HeadlessOpened`] once the context and renderer have finished initializing or a
    /// [`Event::WindowOrHeadlessOpenError`] if it failed.
    pub fn open_headless(&mut self, request: HeadlessRequest);

    /// Close the window or headless surface.
    ///
    /// All documents associated with the window or surface are also closed.
    pub fn close_window(&mut self, id: WindowId);

    /// Set window title.
    pub fn set_title(&mut self, id: WindowId, title: String);

    /// Set window visible.
    pub fn set_visible(&mut self, id: WindowId, visible: bool);

    /// Set if the window is "top-most".
    pub fn set_always_on_top(&mut self, id: WindowId, always_on_top: bool);

    /// Set if the user can drag-move the window when it is in `Normal` mode.
    pub fn set_movable(&mut self, id: WindowId, movable: bool);

    /// Set if the user can resize the window when it is in `Normal` mode.
    pub fn set_resizable(&mut self, id: WindowId, resizable: bool);

    /// Set the window taskbar icon visibility.
    pub fn set_taskbar_visible(&mut self, id: WindowId, visible: bool);

    /// Set the window parent and if `self` blocks the parent events while open (`modal`).
    pub fn set_parent(&mut self, id: WindowId, parent: Option<WindowId>, modal: bool);

    /// Set the window state, position, size.
    pub fn set_state(&mut self, id: WindowId, state: WindowStateAll);

    /// Set the headless surface or document area size (viewport size).
    pub fn set_headless_size(&mut self, id: WindowId, document_id: DocumentId, size: DipSize, scale_factor: f32);

    /// Set the window icon.
    pub fn set_icon(&mut self, id: WindowId, icon: Option<ImageId>);

    /// Set the window cursor icon and visibility.
    pub fn set_cursor(&mut self, id: WindowId, icon: Option<CursorIcon>);

    /// Sets the user attention request indicator, the indicator is cleared when the window is focused or
    /// if canceled by setting to `None`.
    pub fn set_focus_indicator(&mut self, id: WindowId, indicator: Option<FocusIndicator>);

    /// Brings the window to the front and sets input focus.
    ///
    /// Sends an [`Event::FocusChanged`] if the window is focused, the request can be ignored by the window manager, or if the
    /// window is not visible, minimized or already focused.
    ///
    /// This request can steal focus from other apps disrupting the user, be careful with it.
    pub fn focus_window(&mut self, id: WindowId);

    /// Cache an image resource.
    ///
    /// The image is decoded asynchronously, the events [`Event::ImageMetadataLoaded`], [`Event::ImageLoaded`]
    /// or [`Event::ImageLoadError`] will be send when the image is ready for use or failed.
    ///
    /// The `data` handle must contain the full image data already, it will be dropped after the image finishes decoding.
    ///
    /// Images are shared between renderers, to use an image in a window you must first call [`use_image`]
    /// this will register the image data with the renderer.
    ///
    /// [`use_image`]: Api::use_image
    pub fn add_image(&mut self, format: ImageDataFormat, data: IpcBytes, max_decoded_size: u64) -> ImageId;

    /// Cache an image from data that has not fully loaded.
    ///
    /// If the view-process implementation supports **progressive decoding** it will start decoding the image
    /// as more data is received, otherwise it will collect all data first and then [`add_image`]. Each
    /// `data` package is the continuation of the previous call, send an empty package to indicate finish.
    ///
    /// The events [`Event::ImageMetadataLoaded`], [`Event::ImageLoaded`] or [`Event::ImageLoadError`] will
    /// be send while decoding.
    ///
    /// [`add_image`]: Api::add_image
    pub fn add_image_pro(&mut self, format: ImageDataFormat, data: IpcBytesReceiver, max_decoded_size: u64) -> ImageId;

    /// Remove an image from cache.
    ///
    /// Note that if the image is is use in a renderer it will remain in memory until [`delete_image_use`] is
    /// called or the renderer is deinited by closing the window.
    ///
    /// [`delete_image_use`]: Api::delete_image_use
    pub fn forget_image(&mut self, id: ImageId);

    /// Add an image resource to the window renderer.
    ///
    /// Returns the new image key. If the `image_id` is not loaded returns the [`DUMMY`] image key.
    ///
    /// [`DUMMY`]: ImageKey::DUMMY
    pub fn use_image(&mut self, id: WindowId, image_id: ImageId) -> ImageKey;

    /// Replace the image resource in the window renderer.
    ///
    /// The [`ImageKey`] will be associated with the new [`ImageId`].
    pub fn update_image_use(
        &mut self,
        id: WindowId,
        key: ImageKey,
        image_id: ImageId,
    );

    /// Delete the image resource in the window renderer.
    pub fn delete_image_use(&mut self, id: WindowId, key: ImageKey);

    /// Returns a list of image decoders supported by this implementation.
    ///
    /// Each string is the lower-case file extension.
    pub fn image_decoders(&mut self) -> Vec<String>;

    /// Returns a list of image encoders supported by this implementation.
    ///
    /// Each string is the lower-case file extension.
    pub fn image_encoders(&mut self) -> Vec<String>;

    /// Encode the image into the `format`.
    ///
    /// The format must be one of the values returned by [`image_encoders`].
    ///
    /// Returns immediately. The encoded data will be send as the event
    /// [`Event::ImageEncoded`] or [`Event::ImageEncodeError`].
    ///
    /// [`image_encoders`]: Api::image_encoders
    pub fn encode_image(&mut self, id: ImageId, format: String);

    /// Add a raw font resource to the window renderer.
    ///
    /// Returns the new font key.
    pub fn add_font(&mut self, id: WindowId, bytes: IpcBytes, index: u32) -> FontKey;

    /// Delete the font resource in the window renderer.
    pub fn delete_font(&mut self, id: WindowId, key: FontKey);

    /// Add a font instance to the window renderer.
    ///
    /// Returns the new instance key.
    pub fn add_font_instance(
        &mut self,
        id: WindowId,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<webrender_api::FontInstanceOptions>,
        plataform_options: Option<webrender_api::FontInstancePlatformOptions>,
        variations: Vec<webrender_api::FontVariation>,
    ) -> FontInstanceKey;

    /// Delete a font instance.
    pub fn delete_font_instance(&mut self, id: WindowId, instance_key: FontInstanceKey);

    /// Sets if the headed window is in *capture-mode*. If `true` the resources used to capture
    /// a screenshot are kept in memory to be reused in the next screenshot capture.
    ///
    /// Headless surfaces are always capture-mode enabled.
    pub fn set_capture_mode(&mut self, id: WindowId, enable: bool);

    /// Create a new image resource from the current rendered frame.
    ///
    /// Returns immediately if an [`Event::FrameImageReady`] will be send when the image is ready.
    /// Returns `0` if the window is not found.
    pub fn frame_image(&mut self, id: WindowId) -> ImageId;

    /// Create a new image from a selection of the current rendered frame.
    ///
    /// Returns immediately if an [`Event::FrameImageReady`] will be send when the image is ready.
    /// Returns `0` if the window is not found.
    pub fn frame_image_rect(&mut self, id: WindowId, rect: PxRect) -> ImageId;

    /// Set the video mode used when the window is in exclusive fullscreen.
    pub fn set_video_mode(&mut self, id: WindowId, mode: VideoMode);

    ///  Render a new frame.
    pub fn render(&mut self, id: WindowId, frame: FrameRequest);

    /// Update the current frame and re-render it.
    pub fn render_update(&mut self, id: WindowId, frame: FrameUpdateRequest);

    /// Used for testing respawn.
    #[cfg(debug_assertions)]
    pub fn crash(&mut self);
}

pub(crate) type AnyResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
