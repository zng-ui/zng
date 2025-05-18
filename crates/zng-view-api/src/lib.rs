#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! The View Process API.
//!
//! Zng isolates all render and windowing related code to a different process (the view-process), this crate
//! provides the API that must be implemented to create a view-process backend, plus the [`Controller`] that
//! can be used from an app-process to spawn and communicate with a view-process.
//!
//! # VERSION
//!
//! The [`VERSION`] of this crate must match exactly in both *App-Process* and *View-Process*, otherwise a runtime
//! panic error is generated.
//!
//! # Same Process Patch
//!
//! Dynamically loaded same process implementers must propagate a [`StaticPatch`], otherwise the view will not connect.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(missing_docs)]
#![warn(unused_extern_crates)]

use drag_drop::{DragDropData, DragDropEffect, DragDropError};
#[cfg(ipc)]
use serde::{Deserialize, Serialize};

/// The *App Process* and *View Process* must be build using the same exact version and this is
/// validated during run-time, causing a panic if the versions don't match.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod access;
pub mod api_extension;
pub mod clipboard;
pub mod config;
pub mod dialog;
pub mod display_list;
pub mod drag_drop;
pub mod font;
pub mod image;
pub mod ipc;
pub mod keyboard;
pub mod mouse;
pub mod touch;
pub mod window;

mod types;
pub use types::*;

mod app_process;
pub use app_process::*;

mod view_process;
pub use view_process::*;
use zng_txt::Txt;

use std::fmt;

use api_extension::{ApiExtensionId, ApiExtensionPayload};
use clipboard::{ClipboardData, ClipboardError};
use dialog::DialogId;
use font::{FontFaceId, FontId, FontOptions, FontVariationName};
use image::{ImageId, ImageMaskMode, ImageRequest, ImageTextureId};
use ipc::{IpcBytes, IpcBytesReceiver};
use window::WindowId;
use zng_unit::{DipPoint, DipRect, DipSize, Factor, Px, PxRect};

/// Packaged API request.
#[derive(Debug)]
#[cfg_attr(ipc, derive(Serialize, Deserialize))]
pub struct Request(RequestData);
impl Request {
    /// Returns `true` if the request can only be made after the *init* event.
    pub fn must_be_connected(&self) -> bool {
        !matches!(&self.0, RequestData::init { .. })
    }

    /// Returns `true` if the request represents a new frame or frame update for the window with the same wait ID.
    pub fn is_frame(&self, window_id: WindowId, wait_id: Option<window::FrameWaitId>) -> bool {
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
#[cfg_attr(ipc, derive(Serialize, Deserialize))]
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
        #[cfg_attr(ipc, derive(Serialize, Deserialize))]
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
        #[cfg_attr(ipc, derive(Serialize, Deserialize))]
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
    /// Sends an [`Event::Inited`] once the view is completely connected.
    /// Other methods may only be called after this event.
    fn init(&mut self, vp_gen: ViewProcessGen, is_respawn: bool, device_events: bool, headless: bool);

    /// Called once after exit, if running in a managed external process it will be killed after this call.
    fn exit(&mut self);

    /// Open a window.
    ///
    /// Sends an [`Event::WindowOpened`] once the window, context and renderer have finished initializing or a
    /// [`Event::WindowOrHeadlessOpenError`] if it failed.
    pub fn open_window(&mut self, request: window::WindowRequest);

    /// Open a headless surface.
    ///
    /// This is a real renderer but not connected to any window, you can requests pixels to get the
    /// rendered frames.
    ///
    /// Sends an [`Event::HeadlessOpened`] once the context and renderer have finished initializing or a
    /// [`Event::WindowOrHeadlessOpenError`] if it failed.
    pub fn open_headless(&mut self, request: window::HeadlessRequest);

    /// Close the window or headless surface.
    ///
    /// All documents associated with the window or surface are also closed.
    pub fn close(&mut self, id: WindowId);

    /// Set window title.
    pub fn set_title(&mut self, id: WindowId, title: Txt);

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

    /// Bring the window to the Z top, without focusing it.
    pub fn bring_to_top(&mut self, id: WindowId);

    /// Set the window state, position, size.
    pub fn set_state(&mut self, id: WindowId, state: window::WindowStateAll);

    /// Set the headless surface or document area size (viewport size).
    pub fn set_headless_size(&mut self, id: WindowId, size: DipSize, scale_factor: Factor);

    /// Set the window icon, the icon image must be loaded.
    pub fn set_icon(&mut self, id: WindowId, icon: Option<ImageId>);

    /// Set the window cursor icon and visibility.
    pub fn set_cursor(&mut self, id: WindowId, cursor: Option<window::CursorIcon>);

    /// Set the window cursor to a custom image.
    ///
    /// Falls back to cursor icon if not supported or if set to `None`.
    pub fn set_cursor_image(&mut self, id: WindowId, cursor: Option<window::CursorImage>);

    /// Sets the user attention request indicator, the indicator is cleared when the window is focused or
    /// if canceled by setting to `None`.
    pub fn set_focus_indicator(&mut self, id: WindowId, indicator: Option<window::FocusIndicator>);

    /// Set enabled window chrome buttons.
    pub fn set_enabled_buttons(&mut self, id: WindowId, buttons: window::WindowButton);

    /// Brings the window to the front and sets input focus.
    ///
    /// Sends an [`Event::FocusChanged`] if the window is focused, the request can be ignored by the window manager, or if the
    /// window is not visible, minimized or already focused.
    ///
    /// This request can steal focus from other apps disrupting the user, be careful with it.
    pub fn focus(&mut self, id: WindowId) -> FocusResult;

    /// Moves the window with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed immediately before this function is called.
    pub fn drag_move(&mut self, id: WindowId);

    /// Resizes the window with the left mouse button until the button is released.
    ///
    /// There's no guarantee that this will work unless the left mouse button was pressed immediately before this function is called.
    pub fn drag_resize(&mut self, id: WindowId, direction: window::ResizeDirection);

    /// Open the system title bar context menu.
    pub fn open_title_bar_context_menu(&mut self, id: WindowId, position: DipPoint);

    /// Cache an image resource.
    ///
    /// The image is decoded asynchronously, the events [`Event::ImageMetadataLoaded`], [`Event::ImageLoaded`]
    /// or [`Event::ImageLoadError`] will be send when the image is ready for use or failed.
    ///
    /// The [`ImageRequest::data`] handle must contain the full image data already, it will be dropped after the image finishes decoding.
    ///
    /// Images are shared between renderers, to use an image in a window you must first call [`use_image`]
    /// this will register the image data with the renderer.
    ///
    /// [`use_image`]: Api::use_image
    pub fn add_image(&mut self, request: ImageRequest<IpcBytes>) -> ImageId;

    /// Cache an image from data that has not fully loaded.
    ///
    /// If the view-process implementation supports **progressive decoding** it will start decoding the image
    /// as more data is received, otherwise it will collect all data first and then [`add_image`]. Each
    /// [`ImageRequest::`data`] package is the continuation of the previous call, send an empty package to indicate finish.
    ///
    /// The events [`Event::ImageMetadataLoaded`], [`Event::ImageLoaded`] or [`Event::ImageLoadError`] will
    /// be send while decoding.
    ///
    /// [`add_image`]: Api::add_image
    pub fn add_image_pro(&mut self, request: ImageRequest<IpcBytesReceiver>) -> ImageId;

    /// Remove an image from cache.
    ///
    /// Note that if the image is in use in a renderer it will remain in memory until [`delete_image_use`] is
    /// called or the renderer is deinited by closing the window.
    ///
    /// [`delete_image_use`]: Api::delete_image_use
    pub fn forget_image(&mut self, id: ImageId);

    /// Add an image resource to the window renderer.
    ///
    /// Returns the new image texture ID. If the `image_id` is not loaded returns the [`INVALID`] image ID.
    ///
    /// [`INVALID`]: ImageTextureId::INVALID
    pub fn use_image(&mut self, id: WindowId, image_id: ImageId) -> ImageTextureId;

    /// Replace the image resource in the window renderer.
    ///
    /// The [`ImageTextureId`] will be associated with the new [`ImageId`].
    pub fn update_image_use(&mut self, id: WindowId, texture_id: ImageTextureId, image_id: ImageId);

    /// Delete the image resource in the window renderer.
    pub fn delete_image_use(&mut self, id: WindowId, texture_id: ImageTextureId);

    /// Returns a list of image decoders supported by this implementation.
    ///
    /// Each string is the lower-case file extension.
    pub fn image_decoders(&mut self) -> Vec<Txt>;

    /// Returns a list of image encoders supported by this implementation.
    ///
    /// Each string is the lower-case file extension.
    pub fn image_encoders(&mut self) -> Vec<Txt>;

    /// Encode the image into the `format`.
    ///
    /// The format must be one of the values returned by [`image_encoders`].
    ///
    /// Returns immediately. The encoded data will be send as the event
    /// [`Event::ImageEncoded`] or [`Event::ImageEncodeError`].
    ///
    /// [`image_encoders`]: Api::image_encoders
    pub fn encode_image(&mut self, id: ImageId, format: Txt);

    /// Add a raw font resource to the window renderer.
    ///
    /// Returns the new font key.
    pub fn add_font_face(&mut self, id: WindowId, bytes: IpcBytes, index: u32) -> FontFaceId;

    /// Delete the font resource in the window renderer.
    pub fn delete_font_face(&mut self, id: WindowId, font_face_id: FontFaceId);

    /// Add a sized font to the window renderer.
    ///
    /// Returns the new fond ID.
    pub fn add_font(
        &mut self,
        id: WindowId,
        font_face_id: FontFaceId,
        glyph_size: Px,
        options: FontOptions,
        variations: Vec<(FontVariationName, f32)>,
    ) -> FontId;

    /// Delete a font instance.
    pub fn delete_font(&mut self, id: WindowId, font_id: FontId);

    /// Sets if the headed window is in *capture-mode*. If `true` the resources used to capture
    /// a screenshot may be kept in memory to be reused in the next screenshot capture.
    ///
    /// Note that capture must still be requested in each frame request.
    pub fn set_capture_mode(&mut self, id: WindowId, enable: bool);

    /// Create a new image resource from the current rendered frame.
    ///
    /// If `mask` is set captures an A8 mask, otherwise captures a full BGRA8 image.
    ///
    /// Returns immediately if an [`Event::FrameImageReady`] will be send when the image is ready.
    /// Returns `0` if the window is not found.
    pub fn frame_image(&mut self, id: WindowId, mask: Option<ImageMaskMode>) -> ImageId;

    /// Create a new image from a selection of the current rendered frame.
    ///
    /// If `mask` is set captures an A8 mask, otherwise captures a full BGRA8 image.
    ///
    /// Returns immediately if an [`Event::FrameImageReady`] will be send when the image is ready.
    /// Returns `0` if the window is not found.
    pub fn frame_image_rect(&mut self, id: WindowId, rect: PxRect, mask: Option<ImageMaskMode>) -> ImageId;

    /// Set the video mode used when the window is in exclusive fullscreen.
    pub fn set_video_mode(&mut self, id: WindowId, mode: window::VideoMode);

    /// Render a new frame.
    pub fn render(&mut self, id: WindowId, frame: window::FrameRequest);

    /// Update the current frame and re-render it.
    pub fn render_update(&mut self, id: WindowId, frame: window::FrameUpdateRequest);

    /// Update the window's accessibility info tree.
    pub fn access_update(&mut self, id: WindowId, update: access::AccessTreeUpdate);

    /// Shows a native message dialog for the window.
    ///
    /// Returns an ID that identifies the response event.
    pub fn message_dialog(&mut self, id: WindowId, dialog: dialog::MsgDialog) -> DialogId;

    /// Shows a native file/folder picker for the window.
    ///
    /// Returns the ID that identifies the response event.
    pub fn file_dialog(&mut self, id: WindowId, dialog: dialog::FileDialog) -> DialogId;

    /// Get the clipboard content that matches the `data_type`.
    pub fn read_clipboard(&mut self, data_type: clipboard::ClipboardType) -> Result<ClipboardData, ClipboardError>;

    /// Set the clipboard content.
    pub fn write_clipboard(&mut self, data: ClipboardData) -> Result<(), ClipboardError>;

    /// Start a drag and drop operation, if the window is pressed.
    pub fn start_drag_drop(
        &mut self,
        id: WindowId,
        data: Vec<DragDropData>,
        allowed_effects: DragDropEffect,
    ) -> Result<DragDropId, DragDropError>;

    /// Cancel a drag and drop operation.
    pub fn cancel_drag_drop(&mut self, id: WindowId, drag_id: DragDropId);

    /// Notify the drag source of what effect was applied for a received drag&drop.
    pub fn drag_dropped(&mut self, id: WindowId, drop_id: DragDropId, applied: DragDropEffect);

    /// Enable or disable IME by setting a cursor area.
    ///
    /// In mobile platforms also shows the software keyboard for `Some(_)` and hides it for `None`.
    pub fn set_ime_area(&mut self, id: WindowId, area: Option<DipRect>);

    /// Attempt to set a system wide shutdown warning associated with the window.
    ///
    /// Operating systems that support this show the `reason` in a warning for the user, it must be a short text
    /// that identifies the critical operation that cannot be cancelled.
    ///
    /// Note that there is no guarantee that the view-process or operating system will actually set a block, there
    /// is no error result because operating systems can silently ignore block requests at any moment, even after
    /// an initial successful block.
    ///
    /// Set to an empty text to remove the warning.
    pub fn set_system_shutdown_warn(&mut self, id: WindowId, reason: Txt);

    /// Licenses that may be required to be displayed in the app about screen.
    ///
    /// This is specially important for prebuilt view users, as the tools that scrap licenses
    /// may not find the prebuilt dependencies.
    pub fn third_party_licenses(&mut self) -> Vec<zng_tp_licenses::LicenseUsed>;

    /// Call the API extension.
    ///
    /// The `extension_id` is the index of an extension in the extensions list provided by the view-process on init.
    /// The `extension_request` is any data required by the extension.
    ///
    /// Returns the extension response or [`ApiExtensionPayload::unknown_extension`] if the `extension_id` is
    /// not on the list, or [`ApiExtensionPayload::invalid_request`] if the `extension_request` is not in a
    /// format expected by the extension.
    pub fn app_extension(&mut self, extension_id: ApiExtensionId, extension_request: ApiExtensionPayload) -> ApiExtensionPayload;

    /// Call the API extension.
    ///
    /// This is similar to [`Api::app_extension`], but is targeting the instance of an extension associated
    /// with the `id` window or headless surface.
    pub fn window_extension(
        &mut self,
        id: WindowId,
        extension_id: ApiExtensionId,
        extension_request: ApiExtensionPayload,
    ) -> ApiExtensionPayload;

    /// Call the API extension.
    ///
    /// This is similar to [`Api::app_extension`], but is targeting the instance of an extension associated
    /// with the `id` renderer.
    pub fn render_extension(
        &mut self,
        id: WindowId,
        extension_id: ApiExtensionId,
        extension_request: ApiExtensionPayload,
    ) -> ApiExtensionPayload;
}

pub(crate) type AnyResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;
