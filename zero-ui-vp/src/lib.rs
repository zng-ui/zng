//! Zero-Ui View Process.
//!
//! Zero-Ui isolates all OpenGL and windowing related code to a different process so it can recover from graphics driver errors.
//! This crate contains the `glutin` and `webrender` code that interacts with the actual system. Communication
//! with the app process is done using the `ipc-channel` crate.
//!
//! # VERSION
//!
//! The [`VERSION`] of the `zero-ui-vp` dependency must match in both *App-Process* and *View-Process*, otherwise a runtime
//! panic error is generated. Usually both processes are initialized from the same executable so this is not a problem.
//!
//! # `webrender_api`
//!
//! You must use the `webrender_api` that is re-exported as the [`webrender_api`] module. This is because Mozilla
//! does not follow the crate versioning and publishing conventions, so we depend on `webrender` as a git submodule.
//! The *version* re-exported is, usually, the latest commit that was included in the latest Firefox stable release.
//!
//! # Features
//!
//! The crate features are documented in the `zero-ui-vp/Cargo.toml` file, the most important one is the `"full"` feature
//! that is enabled by default, but can be removed to only build the *client* API code that can be used in a *App-Process*
//! that is not also the *View-Process*.

#![allow(unused_parens)]
#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![cfg_attr(doc_nightly, feature(doc_cfg))]

use std::time::Duration;

#[doc(inline)]
pub use webrender_api;

/// ** Debug Docs Only**
#[cfg(all(doc, debug_assertions, feature = "full"))]
#[doc(inline)]
pub use webrender;

use serde::{Deserialize, Serialize};

mod util;

#[cfg(feature = "full")]
mod view_process;
#[cfg(feature = "full")]
pub use view_process::*;

#[cfg(feature = "full")]
mod config;
#[cfg(feature = "full")]
use config::*;
#[cfg(feature = "full")]
mod headless;
#[cfg(feature = "full")]
mod window;

mod app_process;
mod ipc;
mod types;
pub mod units;

pub use app_process::*;
pub use types::*;

use webrender_api::{DynamicProperties, Epoch, FontInstanceKey, FontKey, HitTestResult, IdNamespace, ImageKey, PipelineId};

use crate::units::*;

const SERVER_NAME_VAR: &str = "ZERO_UI_WR_SERVER";
const MODE_VAR: &str = "ZERO_UI_WR_MODE";

/// The *App Process* and *View Process* must be build using the same exact version of `zero-ui-vp` and this is
/// validated during run-time, causing a panic if the versions don't match. Usually the same executable is used
/// for both processes so this is not a problem.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Declares the `Request` and `Response` enums, and two methods in `Controller` and `ViewApp`, in the
/// controller it packs and sends the request and receives and unpacks the response. In the view it implements
/// the method.
macro_rules! declare_ipc {
    (
        $(
            $(#[$meta:meta])*
            $vis:vis fn $method:ident(&mut $self:ident, $ctx:ident: &Context $(, $input:ident : $RequestType:ty)* $(,)?) -> $ResponseType:ty {
                $($impl:tt)*
            }
        )*
    ) => {
        #[derive(Debug, Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        #[allow(clippy::large_enum_variant)]
        #[repr(u32)]
        pub(crate) enum Request {
            $(
                $(#[$meta])*
                $method { $($input: $RequestType),* },
            )*
        }

        #[derive(Debug, Serialize, Deserialize)]
        #[allow(non_camel_case_types)]
        #[repr(u32)]
        pub(crate) enum Response {
            $(
                $(#[$meta])*
                $method($ResponseType),
            )*
        }

        #[allow(unused_parens)]
        impl Controller {
            $(
                $(#[$meta])*
                #[allow(clippy::too_many_arguments)]
                $vis fn $method(&mut self $(, $input: $RequestType)*) -> Result<$ResponseType> {
                    match self.talk(Request::$method { $($input),* })? {
                        Response::$method(r) => Ok(r),
                        _ => panic!("view-process did not respond correctly")
                    }
                }
            )*
        }

        #[allow(unused_parens)]
        #[cfg(feature = "full")]
        impl<E: AppEventSender> ViewApp<E> {
            pub fn on_request(&mut self, ctx: &Context<E>, request: Request) {
                match request {
                    $(
                        #[allow(unused_doc_comments)]
                        $(#[$meta])* // for the cfg
                        Request::$method { $($input),* } => {
                            let r = self.$method(ctx, $($input),*);
                            self.respond(Response::$method(r));
                        }
                    )*
                }
            }

            $(
                $(#[$meta])*
                #[allow(clippy::too_many_arguments)]
                fn $method(&mut $self, $ctx: &Context<E> $(, $input: $RequestType)*) -> $ResponseType {
                    $($impl)*
                }
            )*
        }
    };
}
#[cfg(feature = "full")]
macro_rules! with_window_or_headless {
    ($self:ident, $id:ident, $default:expr, |$w:ident| $($expr:tt)+) => {
        if !$self.started {
            panic!("expected `self.started`");
        } else if let Some($w) = $self.windows.iter_mut().find(|w| w.id() == $id) {
            $($expr)+
        } else if let Some($w) = $self.headless_views.iter_mut().find(|w| w.id() == $id) {
            $($expr)+
        } else {
            $default
        }
    }
}
declare_ipc! {
    fn version(&mut self, _ctx: &Context) -> String {
        crate::VERSION.to_string()
    }

    fn startup(&mut self, _ctx: &Context, gen: ViewProcessGen, device_events: bool, headless: bool) -> bool {
        assert!(!self.started, "view-process already started");

        self.generation = gen;
        self.device_events = device_events;

        assert!(self.headless == headless, "view-process environemt and startup do not agree");

        self.started = true;
        true
    }

    fn exit_same_process(&mut self, _ctx: &Context) -> () {
        let _ = self.event_loop.send(AppEvent::ParentProcessExited);
    }

    /// Returns the primary monitor if there is any or the first available monitor or none if no monitor was found.
    pub fn primary_monitor(&mut self, ctx: &Context) -> Option<(MonId, MonitorInfo)> {
        ctx.window_target
        .primary_monitor()
        .or_else(|| ctx.window_target.available_monitors().next())
        .map(|m| {
            let id = self.monitor_id(&m);
            let mut info = MonitorInfo::from(m);
            info.is_primary = true;
            (id, info)
        })
    }

    /// Returns information about the specific monitor, if it exists.
    pub fn monitor_info(&mut self, ctx: &Context, id: MonId) -> Option<MonitorInfo> {
        self.monitors.iter().find(|(i, _)| *i == id).map(|(_, h)| {
            let mut info = MonitorInfo::from(h);
            info.is_primary = ctx.window_target
                .primary_monitor()
                .map(|p| &p == h)
                .unwrap_or(false);
            info
        })
    }

    /// Returns all available monitors.
    pub fn available_monitors(&mut self, ctx: &Context) -> Vec<(MonId, MonitorInfo)> {
        let primary = ctx.window_target.primary_monitor();

        ctx.window_target
        .available_monitors()
        .map(|m| {
            let id = self.monitor_id(&m);
            let is_primary = primary.as_ref().map(|h|h == &m).unwrap_or(false);
            let mut info = MonitorInfo::from(m);
            info.is_primary = is_primary;
            (id, info)
        })
        .collect()
    }

    /// Open a window.
    ///
    /// Returns the window id, and renderer ids.
    pub fn open_window(
        &mut self,
        ctx: &Context,
        config: WindowConfig,
    ) -> (WinId, IdNamespace, PipelineId) {
        assert!(self.started);

        let mut id = self.window_id_count.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.window_id_count = id;

        let window = window::ViewWindow::new(ctx, self.generation, id, config);
        let namespace = window.namespace_id();
        let pipeline = window.pipeline_id();

        self.windows.push(window);

        (id, namespace, pipeline)
    }

    /// Open a headless surface.
    ///
    /// This is a real renderer but not connected to any window, you can requests pixels to get the
    /// rendered frames.
    ///
    /// The surface is identified with a "window" id, but no window is created, also returns the renderer ids.
    pub fn open_headless(&mut self, ctx: &Context, config: HeadlessConfig) -> (WinId, IdNamespace, PipelineId) {
        assert!(self.started);

        let mut id = self.window_id_count.wrapping_add(1);
        if id == 0 {
            id = 1;
        }
        self.window_id_count = id;

        let view = headless::ViewHeadless::new(ctx, self.generation, id, config);
        let namespace = view.namespace_id();
        let pipeline = view.pipeline_id();

        self.headless_views.push(view);

        (id, namespace, pipeline)
    }

    /// Close the window or headless surface.
    pub fn close_window(&mut self, _ctx: &Context, id: WinId) -> () {
        assert!(self.started);

        if let Some(i) = self.windows.iter().position(|w|w.id() == id) {
            self.windows.remove(i);
        } else if let Some(i) = self.headless_views.iter().position(|h|h.id() == id) {
            self.headless_views.remove(i);
        }
    }

    /// Reads the default text anti-aliasing.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return `TextAntiAliasing::Subpixel`.
    pub fn text_aa(&mut self, _ctx: &Context) -> TextAntiAliasing {
        text_aa()
    }

    /// Reads the system "double-click" config.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return [`MultiClickConfig::default`].
    pub fn multi_click_config(&mut self, _ctx: &Context) -> MultiClickConfig {
        multi_click_config()
    }

    /// Returns `true` if animations are enabled in the operating system.
    ///
    /// People with photosensitive epilepsy usually disable animations system wide.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return `true`.
    pub fn animation_enabled(&mut self, _ctx: &Context) -> bool {
        animation_enabled()
    }

    /// Retrieves the keyboard repeat-delay setting from the operating system.
    ///
    /// If the user holds a key pressed a new key-press event will happen every time this delay is elapsed.
    /// Note, depending on the hardware the real delay can be slightly different.
    ///
    /// There is no repeat flag in the `winit` key press event, so as a general rule we consider a second key-press
    /// without any other keyboard event within the window of time of twice this delay as a repeat.
    ///
    /// This delay can also be used as the text-boxes caret blink rate.
    ///
    /// # TODO
    ///
    /// Only implemented for Windows, other systems return `600ms`.
    pub fn key_repeat_delay(&mut self, _ctx: &Context) -> Duration {
        key_repeat_delay()
    }

    /// Set window title.
    pub fn set_title(&mut self, _ctx: &Context, id: WinId, title: String) -> () {
        self.with_window(id, ||(), |w| w.set_title(title))
    }

    /// Set window visible.
    pub fn set_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> () {
        self.with_window(id, ||(), |w| w.set_visible(visible))
    }

    /// Set if the window is "top-most".
    pub fn set_always_on_top(&mut self, _ctx: &Context, id: WinId, always_on_top: bool) -> () {
        self.with_window(id, ||(), |w| w.set_always_on_top(always_on_top))
    }

    /// Set if the user can drag-move the window.
    pub fn set_movable(&mut self, _ctx: &Context, id: WinId, movable: bool) -> () {
        self.with_window(id, ||(), |w| w.set_movable(movable))
    }

    /// Set if the user can resize the window.
    pub fn set_resizable(&mut self, _ctx: &Context, id: WinId, resizable: bool) -> () {
        self.with_window(id, ||(), |w| w.set_resizable(resizable))
    }

    /// Set the window taskbar icon visibility.
    pub fn set_taskbar_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> () {
        self.with_window(id, ||(), |w| w.set_taskbar_visible(visible))
    }

    /// Set the window parent and if `self` blocks the parent events while open (`modal`).
    pub fn set_parent(&mut self, _ctx: &Context, id: WinId, parent: Option<WinId>, modal: bool) -> () {
        if let Some(parent_id) = parent {
            if let Some(parent_id) = self.windows.iter().find(|w|w.id() == parent_id).map(|w|w.actual_id()) {
                self.with_window(id, ||(), |w|w.set_parent(Some(parent_id), modal))
            } else {
                self.with_window(id, ||(), |w| w.set_parent(None, modal))
            }
        } else {
            self.with_window(id, ||(), |w| w.set_parent(None, modal))
        }
    }

    /// Set if the window is see-through.
    pub fn set_transparent(&mut self, _ctx: &Context, id: WinId, transparent: bool) -> () {
        self.with_window(id, ||(), |w| w.set_transparent(transparent))
    }

    /// Set the window system border and title visibility.
    pub fn set_chrome_visible(&mut self, _ctx: &Context, id: WinId, visible: bool) -> () {
        self.with_window(id, ||(), |w|w.set_chrome_visible(visible))
    }

    /// Set the window top-left offset, includes the window chrome (outer-position).
    pub fn set_position(&mut self, _ctx: &Context, id: WinId, pos: DipPoint) -> () {
        if self.with_window(id, ||false, |w|w.set_outer_pos(pos)) {
            self.notify(Ev::WindowMoved(id, pos, EventCause::App));
        }
    }

    /// Set the window content area size (inner-size).
    pub fn set_size(&mut self, _ctx: &Context, id: WinId, size: DipSize, frame: FrameRequest) -> () {
        let frame_id = frame.id;
        let (resized, rendered) = self.with_window(id, ||(false, false), |w|w.resize_inner(size, frame));
        if resized {
            self.notify(Ev::WindowResized(id, size, EventCause::App));
            if rendered {
                self.notify(Ev::FrameRendered(id, frame_id))
            }
        }
    }

    /// Set the headless surface are size (viewport size).
    pub fn set_headless_size(&mut self, _ctx: &Context, id: WinId, size: DipSize, scale_factor: f32) -> () {
        self.with_headless(id, ||(), |h|h.set_size(size, scale_factor))
    }

    /// Set the window minimum content area size.
    pub fn set_min_size(&mut self, _ctx: &Context, id: WinId, size: DipSize) -> () {
        self.with_window(id, ||(), |w|w.set_min_inner_size(size))
    }
    /// Set the window maximum content area size.
    pub fn set_max_size(&mut self, _ctx: &Context, id: WinId, size: DipSize) -> () {
        self.with_window(id, ||(), |w|w.set_max_inner_size(size))
    }

    /// Set the window icon.
    pub fn set_icon(&mut self, _ctx: &Context, id: WinId, icon: Option<Icon>) -> () {
        self.with_window(id, ||(), |w|w.set_icon(icon))
    }

    /// Gets the root pipeline ID.
    pub fn pipeline_id(&mut self, _ctx: &Context, id: WinId) -> PipelineId {
        with_window_or_headless!(self, id, PipelineId::dummy(), |w|w.pipeline_id())
    }

    /// Gets the resources namespace.
    pub fn namespace_id(&mut self, _ctx: &Context, id: WinId) -> IdNamespace {
        with_window_or_headless!(self, id, IdNamespace(0), |w|w.namespace_id())
    }

    /// Add an image resource to the window renderer.
    ///
    /// Returns the new image key.
    pub fn add_image(&mut self, _ctx: &Context, id: WinId, descriptor: webrender_api::ImageDescriptor, data: ByteBuf) -> ImageKey {
        with_window_or_headless!(self, id, ImageKey::DUMMY, |w| {
            let key = w.generate_image_key();
            let mut txn = webrender::Transaction::new();
            txn.add_image(key, descriptor, webrender_api::ImageData::Raw(std::sync::Arc::new(data.into_vec())), None);
            w.send_transaction(txn);
            key
        })
    }

    /// Replace the image resource in the window renderer.
    pub fn update_image(
        &mut self,
        _ctx: &Context,
        id: WinId,
        key: ImageKey,
        descriptor: webrender_api::ImageDescriptor,
        data: ByteBuf,
        dirty_rect: webrender_api::units::ImageDirtyRect
    ) -> () {
        with_window_or_headless!(self, id, (), |w| {
            let mut txn = webrender::Transaction::new();
            txn.update_image(key, descriptor, webrender_api::ImageData::Raw(std::sync::Arc::new(data.into_vec())), &dirty_rect);
            w.send_transaction(txn);
        })
    }

    /// Delete the image resource in the window renderer.
    pub fn delete_image(&mut self, _ctx: &Context, id: WinId, key: ImageKey) -> () {
        with_window_or_headless!(self, id, (), |w| {
            let mut txn = webrender::Transaction::new();
            txn.delete_image(key);
            w.send_transaction(txn);
        })
    }

    /// Add a raw font resource to the window renderer.
    ///
    /// Returns the new font key.
    pub fn add_font(&mut self, _ctx: &Context, id: WinId, bytes: ByteBuf, index: u32) -> FontKey {
        with_window_or_headless!(self, id, FontKey(IdNamespace(0), 0), |w| {
            let key = w.generate_font_key();
            let mut txn = webrender::Transaction::new();
            txn.add_raw_font(key, bytes.into_vec(), index);
            w.send_transaction(txn);
            key
        })
    }

    /// Delete the font resource in the window renderer.
    pub fn delete_font(&mut self, _ctx: &Context, id: WinId, key: FontKey) -> () {
        with_window_or_headless!(self, id, (), |w| {
            let mut txn = webrender::Transaction::new();
            txn.delete_font(key);
            w.send_transaction(txn);
        })
    }

    /// Add a font instance to the window renderer.
    ///
    /// Returns the new instance key.
    pub fn add_font_instance(
        &mut self,
        _ctx: &Context,
        id: WinId,
        font_key: FontKey,
        glyph_size: Px,
        options: Option<webrender_api::FontInstanceOptions>,
        plataform_options: Option<webrender_api::FontInstancePlatformOptions>,
        variations: Vec<webrender_api::FontVariation>,
    ) -> FontInstanceKey {
        with_window_or_headless!(self, id, FontInstanceKey(IdNamespace(0), 0), |w| {
            let key = w.generate_font_instance_key();
            let mut txn = webrender::Transaction::new();
            txn.add_font_instance(key, font_key, glyph_size.to_wr().get(), options, plataform_options, variations);
            w.send_transaction(txn);
            key
        })
    }

    /// Delete a font instance.
    pub fn delete_font_instance(&mut self, _ctx: &Context, id: WinId, instance_key: FontInstanceKey) -> () {
        with_window_or_headless!(self, id, (), |w| {
            let mut txn = webrender::Transaction::new();
            txn.delete_font_instance(instance_key);
            w.send_transaction(txn);
        })
    }

    /// Gets the window content are size.
    pub fn size(&mut self, _ctx: &Context, id: WinId) -> DipSize {
        with_window_or_headless!(self, id, DipSize::zero(), |w|w.size())
    }

    /// Gets the window content are size.
    pub fn scale_factor(&mut self, _ctx: &Context, id: WinId) -> f32 {
        with_window_or_headless!(self, id, 1.0, |w|w.scale_factor())
    }

    /// In Windows, set if the `Alt+F4` should not cause a window close request and instead generate a key-press event.
    pub fn set_allow_alt_f4(&mut self, _ctx: &Context, id: WinId, allow: bool) -> () {
        self.with_window(id, ||(), |w|w.set_allow_alt_f4(allow))
    }

    /// Read all pixels of the current frame.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels(&mut self, _ctx: &Context, id: WinId) -> FramePixels {
        with_window_or_headless!(self, id, FramePixels::default(), |w|w.read_pixels())
    }

    /// `glReadPixels` a new buffer.
    ///
    /// This is a call to `glReadPixels`, the first pixel row order is bottom-to-top and the pixel type is BGRA.
    pub fn read_pixels_rect(&mut self, _ctx: &Context, id: WinId, rect: PxRect) -> FramePixels {
        with_window_or_headless!(self, id, FramePixels::default(), |w|w.read_pixels_rect(rect))
    }

    /// Get display items of the last rendered frame that intercept the `point`.
    ///
    /// Returns the frame ID and all hits from front-to-back.
    pub fn hit_test(&mut self, _ctx: &Context, id: WinId, point: PxPoint) -> (Epoch, HitTestResult) {
        with_window_or_headless!(self, id, (Epoch(0), HitTestResult::default()), |w|w.hit_test(point))
    }

    /// Set the text anti-aliasing used in the window renderer.
    pub fn set_text_aa(&mut self, _ctx: &Context, id: WinId, aa: TextAntiAliasing) -> () {
        with_window_or_headless!(self, id, (), |w|w.set_text_aa(aa))
    }

    /// Render a new frame.
    pub fn render(&mut self, _ctx: &Context, id: WinId, frame: FrameRequest) -> () {
        with_window_or_headless!(self, id, (), |w|w.render(frame))
    }

    /// Update the current frame and re-render it.
    pub fn render_update(&mut self, _ctx: &Context, id: WinId, updates: DynamicProperties) -> () {
        with_window_or_headless!(self, id, (), |w|w.render_update(updates))
    }

    /// Used for testing respawn.
    #[cfg(debug_assertions)]
    pub fn crash(&mut self, _ctx: &Context) -> () {
        panic!("TEST CRASH")
    }
}
