//! Extensions API
//!
//! Extensions that run in the view-process, with internal access to things like the raw handle of windows or
//! direct access to renderers. These extensions are build on top of the view API extensions as a way to customize
//! the view-process without needing to fork it or re-implement the entire view API from scratch.
//!

use std::rc::Rc;
use std::{any::Any, sync::Arc};

use webrender::api::{
    AsyncBlobImageRasterizer, BlobImageHandler, BlobImageParams, BlobImageRequest, BlobImageResult, DocumentId, ExternalImageId,
    PipelineId, units::TexelRect,
};
use webrender::{DebugFlags, RenderApi};
use zng_task::channel::{ChannelError, IpcBytes};
use zng_task::parking_lot::Mutex;
use zng_unit::{Factor, PxSize};
use zng_view_api::window::RenderMode;
use zng_view_api::{
    Event,
    api_extension::{ApiExtensionId, ApiExtensionName, ApiExtensionPayload, ApiExtensions},
};

use crate::display_list::{DisplayExtensionArgs, DisplayExtensionItemArgs, DisplayExtensionUpdateArgs, DisplayListExtension, SpaceAndClip};

pub use crate::px_wr::{PxToWr, WrToPx};
use crate::util::PxToWinit;

/// The extension API.
pub trait ViewExtension: Send + Any {
    /// Called once at the start of the view-process.
    fn init(&mut self, args: ViewExtensionInitArgs) {
        let _ = args;
    }

    /// Unique name and version of this extension.
    fn name(&self) -> &ApiExtensionName;

    /// Run the extension as an app level command.
    fn command(&mut self, request: ApiExtensionPayload) -> Option<ApiExtensionPayload> {
        let _ = request;
        None
    }

    /// Create a [`WindowExtension`] for a new window instance.
    fn window(&mut self) -> Option<Box<dyn WindowExtension>> {
        None
    }

    /// Create a [`RendererExtension`] for a new renderer instance.
    fn renderer(&mut self) -> Option<Box<dyn RendererExtension>> {
        None
    }

    /// System warning low memory, release unused memory, caches.
    fn low_memory(&mut self) {}

    /// App is being suspended, all graphic resources must be dropped.
    ///
    /// Android and iOS apps can be suspended without fully exiting, all graphic resources must be dropped on suspension, and
    /// any persistent state must be flushed because the app will process will exit if the user does not return to the app.
    ///
    /// Note that [`window`] and [`renderer`] resources are managed by the app and will be dropped automatically, this
    /// callback only needs to drop custom graphic resources.
    ///
    /// [`window`]: Self::window
    /// [`renderer`]: Self::renderer
    fn suspended(&mut self) {}

    /// App resumed from a suspended state.
    ///
    /// Expect [`window`] and [`renderer`] requests to recrate previous instances.
    ///
    /// Note that this is not called on init, only after a suspension.
    ///
    /// [`window`]: Self::window
    /// [`renderer`]: Self::renderer
    fn resumed(&mut self) {}
}

/// Represents a view extension associated with a headed or headless window instance.
pub trait WindowExtension: Any {
    /// Edit attributes for the new window.
    fn configure(&mut self, args: &mut WindowConfigArgs) {
        let _ = args;
    }

    /// Called just after the window is created.
    fn window_inited(&mut self, args: &mut WindowInitedArgs) {
        let _ = args;
    }

    /// If this extension can be dropped after window creation.
    fn is_init_only(&self) -> bool;

    /// Called when a command request is made for the extension and window (window ID).
    fn command(&mut self, args: &mut WindowCommandArgs) -> ApiExtensionPayload {
        let _ = args;
        ApiExtensionPayload::unknown_extension(ApiExtensionId::INVALID)
    }

    /// Called when the window receives an event.
    fn event(&mut self, args: &mut WindowEventArgs) {
        let _ = args;
    }

    /// System warning low memory, release unused memory, caches.
    fn low_memory(&mut self) {}

    /// Called just after the window closes.
    fn window_deinited(&mut self, args: &mut WindowDeinitedArgs) {
        let _ = args;
    }

    /// Cast to `&dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Cast to `&mut dyn Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Represents a view extension associated with a renderer instance.
pub trait RendererExtension: Any {
    /// Edit options for the new renderer.
    fn configure(&mut self, args: &mut RendererConfigArgs) {
        let _ = args;
    }

    /// Called just after the renderer is created.
    fn renderer_inited(&mut self, args: &mut RendererInitedArgs) {
        let _ = args;
    }

    /// If this extension can be dropped after render creation.
    fn is_init_only(&self) -> bool;

    /// Called when a command request is made for the extension and renderer (window ID).
    fn command(&mut self, args: &mut RendererCommandArgs) -> ApiExtensionPayload {
        let _ = args;
        ApiExtensionPayload::unknown_extension(ApiExtensionId::INVALID)
    }

    /// Called when a new display list begins building.
    fn render_start(&mut self, args: &mut RenderArgs) {
        let _ = args;
    }

    /// Called when a display item push for the extension is found.
    fn render_push(&mut self, args: &mut RenderItemArgs) {
        let _ = args;
    }

    /// Called when a display item pop for the extension is found.
    fn render_pop(&mut self, args: &mut RenderItemArgs) {
        let _ = args;
    }

    /// Called when a new display list finishes building.
    ///
    /// The list will be send to the renderer for asynchronous rendering.
    fn render_end(&mut self, args: &mut RenderArgs) {
        let _ = args;
    }

    /// Called when a render-update for the extension is found.
    fn render_update(&mut self, args: &mut RenderUpdateArgs) {
        let _ = args;
    }

    /// Called when Webrender finishes rendering a frame and it is ready for redraw.
    fn frame_ready(&mut self, args: &mut FrameReadyArgs) {
        let _ = args;
    }

    /// Called every time the window or surface redraws, after Webrender has redraw.
    fn redraw(&mut self, args: &mut RedrawArgs) {
        let _ = args;
    }

    /// System warning low memory, release unused memory, caches.
    fn low_memory(&mut self) {}

    /// Called just before the renderer is destroyed.
    fn renderer_deinited(&mut self, args: &mut RendererDeinitedArgs) {
        let _ = args;
    }

    /// Cast to `&dyn Any`.
    fn as_any(&self) -> &dyn Any;

    /// Cast to `&mut dyn Any`.
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

/// Arguments for [`RendererExtension::render_start`] and [`RendererExtension::render_end`].
#[non_exhaustive]
pub struct RenderArgs<'a> {
    /// Id of the new frame.
    pub frame_id: zng_view_api::window::FrameId,

    /// The webrender display list.
    pub list: &'a mut webrender::api::DisplayListBuilder,
    /// Space and clip tracker.
    pub sc: &'a mut SpaceAndClip,

    /// The transaction that will send the display list.
    pub transaction: &'a mut webrender::Transaction,

    /// The window or surface renderer.
    pub renderer: &'a mut webrender::Renderer,
    /// The document ID of the main content.
    pub document_id: DocumentId,
    /// The window or surface render API.
    pub api: &'a mut RenderApi,
    /// External images registry for the `renderer`.
    pub external_images: &'a mut ExternalImages,
}

/// Arguments for [`RendererExtension::render_push`] and [`RendererExtension::render_pop`].
#[non_exhaustive]
pub struct RenderItemArgs<'a> {
    /// Extension index.
    pub extension_id: ApiExtensionId,
    /// Push payload, is empty for pop.
    pub payload: &'a ApiExtensionPayload,
    /// If the display item is reused.
    ///
    /// If `true` the payload is the same as received before any updates, the updated
    /// values must be applied to value deserialized from the payload.
    pub is_reuse: bool,

    /// The webrender display list.
    pub list: &'a mut webrender::api::DisplayListBuilder,
    /// Space and clip tracker.
    pub sc: &'a mut SpaceAndClip,

    /// The transaction that will send the display list.
    pub transaction: &'a mut webrender::Transaction,

    /// The window or surface renderer.
    pub renderer: &'a mut webrender::Renderer,
    /// The target document ID in the renderer.
    pub document_id: DocumentId,
    /// The document ID of the main content.
    pub api: &'a mut RenderApi,
    /// External images registry for the `renderer`.
    pub external_images: &'a mut ExternalImages,
}

/// Arguments for [`RendererExtension::render_update`].
#[non_exhaustive]
pub struct RenderUpdateArgs<'a> {
    /// Extension index.
    pub extension_id: ApiExtensionId,
    /// Update payload.
    pub payload: &'a ApiExtensionPayload,

    /// Set to `true` to rebuild the display list.
    ///
    /// The list will be rebuild using the last full payload received, the extension
    /// must patch in any subsequent updates onto this value.
    pub new_frame: bool,

    /// Webrender binding updates.
    ///
    /// If no other extension and update handlers request a new frame these properties
    /// will be send to Webrender to update the current frame.
    pub properties: &'a mut webrender::api::DynamicProperties,

    /// The transaction that will send the properties update.
    pub transaction: &'a mut webrender::Transaction,

    /// The window or surface renderer.
    pub renderer: &'a mut webrender::Renderer,
    /// The document ID of the main content.
    pub document_id: DocumentId,
    /// The window or surface render API.
    pub api: &'a mut RenderApi,
    /// External images registry for the `renderer`.
    pub external_images: &'a mut ExternalImages,
}

/// Arguments for [`RendererExtension::frame_ready`].
#[non_exhaustive]
pub struct FrameReadyArgs {
    /// Frame that finished rendering and is ready to redraw.
    pub frame_id: zng_view_api::window::FrameId,
    /// If a screen redraw is requested.
    ///
    /// This is `true` if Webrender requested recomposite after rendering the frame, or an
    /// extension set it to `true`. Don't set this to `false`.
    pub redraw: bool,
}

/// Arguments for [`RendererExtension::redraw`].
#[non_exhaustive]
pub struct RedrawArgs<'a> {
    /// Scale factor of the screen or window.
    pub scale_factor: Factor,

    /// Current size of the surface or window content.
    pub size: PxSize,

    /// OpenGL context used by the renderer.
    ///
    /// The context is current, and Webrender has already redraw.
    pub context: &'a mut dyn OpenGlContext,
}

/// Represents a Webrender blob handler that can coexist with other blob handlers on the same renderer.
///
/// This API is very similar to Webrender's `BlobImageHandler`, the only difference is that implementers
/// are expected to detect and ignore requests targeting other blob extensions as multiple extensions may
/// implement blob renderers.
///
/// See [`RendererConfigArgs::blobs`] for more details.
pub trait BlobExtension: Send + Any {
    /// Creates a snapshot of the current state of blob images in the handler.
    fn create_blob_rasterizer(&mut self) -> Box<dyn AsyncBlobRasterizer>;

    /// New blob extension instance of the same type.
    fn create_similar(&self) -> Box<dyn BlobExtension>;

    /// Prepare resources that are not bundled in with the encoded request commands.
    ///
    /// The extension must ignore requests not addressing it.
    fn prepare_resources(&mut self, args: &mut BlobPrepareArgs) {
        let _ = args;
    }

    /// Register a blob image if the request addresses this extension.
    fn add(&mut self, args: &BlobAddArgs);
    /// Update a blob image if the request addresses this extension.
    fn update(&mut self, args: &BlobUpdateArgs);

    /// Remove a blob image if the key was generated by this extension.
    fn delete(&mut self, key: webrender::api::BlobImageKey);

    /// Cleanup any prepared resource for the font.
    fn delete_font(&mut self, key: webrender::api::FontKey) {
        let _ = key;
    }

    /// Cleanup any prepared resource for the font instance.
    fn delete_font_instance(&mut self, key: webrender::api::FontInstanceKey) {
        let _ = key;
    }

    /// Cleanup any state related with the namespace.
    fn clear_namespace(&mut self, namespace: webrender::api::IdNamespace) {
        let _ = namespace;
    }

    /// Sets if multi-threading is allowed.
    ///
    /// The default is `true`, this method is only called on init if multithreading is disabled.
    fn enable_multithreading(&mut self, enable: bool);
}

/// Arguments for [`ViewExtension::init`].
#[non_exhaustive]
pub struct ViewExtensionInitArgs {
    /// Sender of [`Event::ExtensionEvent`] events.
    pub event_sender: ExtensionEventSender,
}

/// Sender of [`Event::ExtensionEvent`] events.
///
/// Available in [`ViewExtensionInitArgs`].
#[derive(Clone)]
pub struct ExtensionEventSender {
    sender: crate::AppEventSender,
    id: ApiExtensionId,
}
impl ExtensionEventSender {
    /// Send the event `payload`.
    pub fn send(&self, payload: ApiExtensionPayload) -> Result<(), ChannelError> {
        self.sender.send(crate::AppEvent::Notify(Event::ExtensionEvent(self.id, payload)))
    }
}

/// Snapshot of a [`BlobExtension`] that can render/copy pixels.
pub trait AsyncBlobRasterizer: Send + Any {
    /// Rasterize the requests addressed for this rasterizer.
    ///
    /// Note that all requests (for all rasterizers) is shared here, the rasterizer must
    /// find their own requests and push responses in the `args`.
    fn rasterize(&mut self, args: &mut BlobRasterizerArgs);
}

/// Arguments for [`BlobExtension::prepare_resources`].
#[non_exhaustive]
pub struct BlobPrepareArgs<'a> {
    /// Webrender services.
    pub services: &'a dyn webrender::api::BlobImageResources,
    /// Requests targeting any of the blob extensions. Each extension must
    /// inspect the requests to find the ones targeting it.
    pub requests: &'a [BlobImageParams],
}

/// Arguments for [`BlobExtension::add`].
#[non_exhaustive]
pub struct BlobAddArgs {
    /// Blob key.
    ///
    /// Blob extension must ignore this request if it did not generate this key.
    pub key: webrender::api::BlobImageKey,
    /// Encoded data.
    pub data: std::sync::Arc<webrender::api::BlobImageData>,

    /// Webrender value.
    pub visible_rect: webrender::api::units::DeviceIntRect,
    /// Webrender value.
    pub tile_size: webrender::api::TileSize,
}

/// Arguments for [`BlobExtension::update`].
#[non_exhaustive]
pub struct BlobUpdateArgs {
    /// Blob key.
    ///
    /// Blob extension must ignore this request if it did not generate this key.
    pub key: webrender::api::BlobImageKey,
    /// Encoded data.
    pub data: std::sync::Arc<webrender::api::BlobImageData>,
    /// Webrender value.
    pub visible_rect: webrender::api::units::DeviceIntRect,
    /// Webrender value.
    pub dirty_rect: webrender::api::units::BlobDirtyRect,
}

/// Arguments for [`AsyncBlobRasterizer::rasterize`].
#[non_exhaustive]
pub struct BlobRasterizerArgs<'a> {
    /// Rasterization requests for all rasterizers.
    ///
    /// The rasterizer must inspect the requests to find the ones targeting it.
    pub requests: &'a [BlobImageParams],
    /// Rasterization request can be schedules in a way that minimizes the risk of
    /// high priority work being enqueued behind it.
    pub low_priority: bool,

    /// A pool of blob tile buffers to mitigate the overhead of allocating and deallocating blob tiles.
    pub tile_pool: &'a mut webrender::api::BlobTilePool,

    /// Rasterization responses.
    ///
    /// Note that `requests` and `responses` are shared by all blob rasterizers, each rasterizer
    /// must inspect the requests and push responses here.
    pub responses: &'a mut Vec<(BlobImageRequest, BlobImageResult)>,
}

/// Arguments for [`WindowExtension::configure`]
#[non_exhaustive]
pub struct WindowConfigArgs<'a> {
    /// Config payload send with the window creation request addressed to this extension.
    ///
    /// Note that this extension will participate in the renderer creation even if there is no config for it.
    pub config: Option<&'a ApiExtensionPayload>,

    /// Window attributes that will be used to build the headed window.
    pub window: Option<&'a mut winit::window::WindowAttributes>,
}

/// Arguments for [`RendererExtension::configure`]
#[non_exhaustive]
pub struct RendererConfigArgs<'a> {
    /// Config payload send with the renderer creation request addressed to this extension.
    ///
    /// Note that this extension will participate in the renderer creation even if there is no config for it.
    pub config: Option<&'a ApiExtensionPayload>,

    /// Webrender options.
    ///
    /// Note that this config is modified by the window and other extensions. Some options
    /// must not be set by extensions:
    ///
    /// * `workers` will be already set by the window, blob rasterizers may clone and use these threads.
    /// * `blob_image_handler` will be set by the window to an object that aggregates
    ///   all extension blob image handlers. Add your own blob handler to `blobs` instead.
    pub options: &'a mut webrender::WebRenderOptions,

    /// Blob extensions.
    ///
    /// Use this API instead of `blob_image_handler` in options to support multiple blob handlers.
    pub blobs: &'a mut Vec<Box<dyn BlobExtension>>,

    /// Winit window if the renderer is associated with a headed window.
    pub window: Option<&'a winit::window::Window>,

    /// OpenGL context that will be used by the new renderer.
    pub context: &'a mut dyn OpenGlContext,
}

/// Arguments for [`RendererExtension::renderer_inited`].
#[non_exhaustive]
pub struct RendererInitedArgs<'a> {
    /// The new renderer.
    ///
    /// Note that some renderer config is set by the window and must not be set by extensions:
    ///
    /// * `set_external_image_handler` will be set by the window to the image cache, you can use `external_images` to
    ///   register external images and textures.
    pub renderer: &'a mut webrender::Renderer,

    /// The API sender connected with the new renderer.
    pub api_sender: &'a webrender::RenderApiSender,

    /// The API used by the window or surface.
    pub api: &'a mut RenderApi,

    /// The document ID of the main content.
    pub document_id: DocumentId,

    /// The pipeline of the main content.
    pub pipeline_id: PipelineId,

    /// Winit window if the renderer is associated with a headed window.
    pub window: Option<&'a winit::window::Window>,

    /// OpenGL context used by the new renderer.
    ///
    /// The context is new and current, only Webrender and previous extensions have interacted with it.
    pub context: &'a mut dyn OpenGlContext,

    /// External images registry for the `renderer`.
    pub external_images: &'a mut ExternalImages,
}

/// Tracks extension external images for a renderer.
#[derive(Default)]
pub struct ExternalImages {
    images: Vec<Arc<crate::image_cache::ImageData>>,
}
impl ExternalImages {
    /// Register an OpenGL texture.
    ///
    /// Returns an `ExternalImageId` that can be used in display lists.
    ///
    /// The id is only valid in the same renderer, and the `texture` must be generated by the same
    /// GL API used by the renderer. Note that you must manage the `texture` lifetime, [`unregister`]
    /// only releases the external image entry.
    ///
    /// [`unregister`]: Self::unregister
    pub fn register_texture(&mut self, uv: TexelRect, texture: gleam::gl::GLuint) -> ExternalImageId {
        self.register(crate::image_cache::ImageData::NativeTexture { uv, texture })
    }

    /// Register a loaded image.
    ///
    /// Returns an `ExternalImageId` that can be used in display lists.
    ///
    /// The `pixels` are held in memory until [`unregister`] or the window is closed. They must be premultiplied BGRA8
    /// or a mask A8.
    ///
    /// # Panics
    ///
    /// Panics if `pixels` length is not equal expected BGRA8 or A8 length.
    ///
    /// [`unregister`]: Self::unregister
    pub fn register_image(&mut self, size: PxSize, is_opaque: bool, pixels: IpcBytes) -> ExternalImageId {
        let expected_len = size.width.0 as usize * size.height.0 as usize;
        assert!(
            pixels.len() == expected_len || pixels.len() == expected_len * 4,
            "pixels must be BGRA8 or A8"
        );
        self.register(crate::image_cache::ImageData::RawData {
            size,
            range: 0..pixels.len(),
            pixels,
            is_opaque,
            density: None,
            mipmap: Mutex::new(Box::new([])),
            stripes: Mutex::new(Box::new([])),
        })
    }

    /// Unregister the image or texture.
    ///
    /// The `id` is invalid after this call, using it in a display list is undefined behavior and
    /// will likely cause access violation or other memory problems.
    pub fn unregister(&mut self, id: ExternalImageId) {
        if let Some(i) = self.images.iter().position(|img| ExternalImageId(Arc::as_ptr(img) as u64) == id) {
            self.images.swap_remove(i);
        }
    }

    fn register(&mut self, img: crate::image_cache::ImageData) -> ExternalImageId {
        let img = Arc::new(img);
        let id = ExternalImageId(Arc::as_ptr(&img) as u64);
        self.images.push(img);
        id
    }
}

/// Arguments for [`RendererExtension::renderer_deinited`].
#[non_exhaustive]
pub struct RendererDeinitedArgs<'a> {
    /// The document ID of the main content, already deinited.
    pub document_id: DocumentId,

    /// The pipeline of the main content, already deinited.
    pub pipeline_id: PipelineId,

    /// Winit window if the renderer is associated with a headed window.
    pub window: Option<&'a winit::window::Window>,

    /// OpenGL context.
    ///
    /// The context is current and Webrender has already deinited, the context will be dropped
    /// after all extensions handle deinit.
    pub context: &'a mut dyn OpenGlContext,
}

/// Arguments for [`WindowExtension::window_inited`].
#[non_exhaustive]
pub struct WindowInitedArgs<'a> {
    /// Underlying winit window.
    pub window: &'a winit::window::Window,

    /// OpenGL context connected to the window or headless surface.
    pub context: &'a mut dyn OpenGlContext,
}

/// Arguments for [`WindowExtension::window_deinited`].
#[non_exhaustive]
pub struct WindowDeinitedArgs<'a> {
    /// Underlying winit window.
    pub window: &'a winit::window::Window,

    /// OpenGL context connected to the window or headless surface.
    pub context: &'a mut dyn OpenGlContext,
}

/// Arguments for [`WindowExtension::command`].
#[non_exhaustive]
pub struct WindowCommandArgs<'a> {
    /// Underlying winit window.
    pub window: &'a winit::window::Window,

    /// OpenGL context connected to the window or headless surface.
    pub context: &'a mut dyn OpenGlContext,

    /// The command request.
    pub request: ApiExtensionPayload,
}

/// Arguments for [`WindowExtension::event`].
#[non_exhaustive]
pub struct WindowEventArgs<'a> {
    /// Underlying winit window.
    pub window: &'a winit::window::Window,

    /// OpenGL context connected to the window or headless surface.
    pub context: &'a mut dyn OpenGlContext,

    /// The event.
    pub event: &'a winit::event::WindowEvent,
}

/// Represents a managed OpenGL context connected to a window or headless surface.
pub trait OpenGlContext {
    /// Context is current on the calling thread.
    fn is_current(&self) -> bool;

    /// Make context current on the calling thread.
    fn make_current(&mut self);

    /// The context.
    fn gl(&self) -> &Rc<dyn gleam::gl::Gl>;

    /// Actual render mode used to create the context.
    fn render_mode(&self) -> RenderMode;

    /// Resize surface.
    fn resize(&mut self, size: PxSize);

    /// If the context runs on the CPU, not a GPU.
    fn is_software(&self) -> bool;

    /// Swap buffers if the context is double-buffered.
    fn swap_buffers(&mut self);
}
impl OpenGlContext for crate::gl::GlContext {
    fn is_current(&self) -> bool {
        self.is_current()
    }

    fn make_current(&mut self) {
        self.make_current()
    }

    fn gl(&self) -> &Rc<dyn gleam::gl::Gl> {
        self.gl()
    }

    fn render_mode(&self) -> RenderMode {
        self.render_mode()
    }

    fn resize(&mut self, size: PxSize) {
        self.resize(size.to_winit())
    }

    fn is_software(&self) -> bool {
        self.is_software()
    }

    fn swap_buffers(&mut self) {
        self.swap_buffers()
    }
}

/// Arguments for [`RendererExtension::command`].
#[non_exhaustive]
pub struct RendererCommandArgs<'a> {
    /// The renderer.
    pub renderer: &'a mut webrender::Renderer,

    /// The render API used by the window or surface.
    pub api: &'a mut RenderApi,

    /// The document ID of the main content.
    pub document_id: DocumentId,

    /// The command request.
    pub request: ApiExtensionPayload,

    /// Winit window if the renderer is associated with a headed window.
    pub window: Option<&'a winit::window::Window>,

    /// OpenGL context associated with the renderer.
    pub context: &'a mut dyn OpenGlContext,

    /// Redraw flag.
    ///
    /// If set to `true` the window is guaranteed to redraw the frame.
    pub redraw: &'a mut bool,
}

/// View extensions register.
pub struct ViewExtensions {
    exts: Vec<Box<dyn ViewExtension>>,
}
impl ViewExtensions {
    /// New empty.
    pub(crate) fn new() -> Self {
        Self { exts: vec![] }
    }

    /// Register an extension with the ID that will be assigned to it.
    ///
    /// The ID is useful for error messages.
    ///
    /// # Panics
    ///
    /// Panics if the name is already registered.
    pub fn register<E: ViewExtension>(&mut self, ext: impl FnOnce(ApiExtensionId) -> E) -> &mut Self {
        let id = ApiExtensionId::from_index(self.exts.len());
        let ext = ext(id);
        assert!(self.id(ext.name()).is_none(), "extension already registered");
        self.exts.push(Box::new(ext));
        self
    }

    /// Returns the extension ID.
    pub fn id(&self, name: &ApiExtensionName) -> Option<ApiExtensionId> {
        self.exts.iter().position(|e| e.name() == name).map(ApiExtensionId::from_index)
    }

    /// Register a command extension with custom encoded messages.
    ///
    /// The `handler` receives the request payload and it's own ID to be used in error messages.
    pub fn command_raw(
        &mut self,
        name: impl Into<ApiExtensionName>,
        handler: impl FnMut(ApiExtensionPayload, ApiExtensionId) -> ApiExtensionPayload + Send + 'static,
    ) -> &mut Self {
        struct CommandExt<F>(ApiExtensionName, ApiExtensionId, F);
        impl<F: FnMut(ApiExtensionPayload, ApiExtensionId) -> ApiExtensionPayload + Send + 'static> ViewExtension for CommandExt<F> {
            fn name(&self) -> &ApiExtensionName {
                &self.0
            }
            fn command(&mut self, request: ApiExtensionPayload) -> Option<ApiExtensionPayload> {
                Some((self.2)(request, self.1))
            }
        }

        self.register(|id| CommandExt(name.into(), id, handler));
        self
    }

    /// Register a command extension.
    ///
    /// The `handler` receives the deserialized request payload and it's own ID to be used in error messages.
    pub fn command<I: serde::de::DeserializeOwned, O: serde::Serialize>(
        &mut self,
        name: impl Into<ApiExtensionName>,
        mut handler: impl FnMut(I, ApiExtensionId) -> O + Send + 'static,
    ) -> &mut Self {
        self.command_raw(name, move |p, id| match p.deserialize::<I>() {
            Ok(p) => {
                let o = handler(p, id);
                ApiExtensionPayload::serialize(&o).unwrap()
            }
            Err(e) => ApiExtensionPayload::invalid_request(id, e),
        })
    }

    /// Register a window extension with its own ID.
    pub fn window<E: WindowExtension>(
        &mut self,
        name: impl Into<ApiExtensionName>,
        new: impl FnMut(ApiExtensionId) -> E + Send + 'static,
    ) -> &mut Self {
        struct WindowExt<F>(ApiExtensionName, ApiExtensionId, F);
        impl<E, F> ViewExtension for WindowExt<F>
        where
            E: WindowExtension,
            F: FnMut(ApiExtensionId) -> E + Send + 'static,
        {
            fn name(&self) -> &ApiExtensionName {
                &self.0
            }

            fn window(&mut self) -> Option<Box<dyn WindowExtension>> {
                Some(Box::new((self.2)(self.1)))
            }
        }
        self.register(move |id| WindowExt(name.into(), id, new));
        self
    }

    /// Register a renderer extension with its own ID.
    pub fn renderer<E: RendererExtension>(
        &mut self,
        name: impl Into<ApiExtensionName>,
        new: impl FnMut(ApiExtensionId) -> E + Send + 'static,
    ) -> &mut Self {
        struct RendererExt<F>(ApiExtensionName, ApiExtensionId, F);
        impl<E, F> ViewExtension for RendererExt<F>
        where
            E: RendererExtension,
            F: FnMut(ApiExtensionId) -> E + Send + 'static,
        {
            fn name(&self) -> &ApiExtensionName {
                &self.0
            }

            fn renderer(&mut self) -> Option<Box<dyn RendererExtension>> {
                Some(Box::new((self.2)(self.1)))
            }
        }
        self.register(move |id| RendererExt(name.into(), id, new));
        self
    }

    pub(crate) fn api_extensions(&self) -> ApiExtensions {
        let mut r = ApiExtensions::new();
        for ext in &self.exts {
            r.insert(ext.name().clone()).unwrap();
        }
        r
    }

    pub(crate) fn call_command(&mut self, id: ApiExtensionId, request: ApiExtensionPayload) -> ApiExtensionPayload {
        let idx = id.index();
        if idx >= self.exts.len() {
            ApiExtensionPayload::unknown_extension(id)
        } else if let Some(r) = self.exts[idx].command(request) {
            r
        } else {
            ApiExtensionPayload::unknown_extension(id)
        }
    }

    pub(crate) fn new_window(&mut self) -> Vec<(ApiExtensionId, Box<dyn WindowExtension>)> {
        self.exts
            .iter_mut()
            .enumerate()
            .filter_map(|(i, e)| e.window().map(|e| (ApiExtensionId::from_index(i), e)))
            .collect()
    }

    pub(crate) fn new_renderer(&mut self) -> Vec<(ApiExtensionId, Box<dyn RendererExtension>)> {
        self.exts
            .iter_mut()
            .enumerate()
            .filter_map(|(i, e)| e.renderer().map(|e| (ApiExtensionId::from_index(i), e)))
            .collect()
    }

    pub(crate) fn init(&mut self, event_sender: &crate::AppEventSender) {
        for (i, ext) in self.exts.iter_mut().enumerate() {
            ext.init(ViewExtensionInitArgs {
                event_sender: ExtensionEventSender {
                    sender: event_sender.clone(),
                    id: ApiExtensionId::from_index(i),
                },
            });
        }
    }

    pub(crate) fn on_low_memory(&mut self) {
        for ext in self.exts.iter_mut() {
            ext.low_memory();
        }
    }

    pub(crate) fn suspended(&mut self) {
        for ext in self.exts.iter_mut() {
            ext.suspended();
        }
    }

    pub(crate) fn resumed(&mut self) {
        for ext in self.exts.iter_mut() {
            ext.resumed();
        }
    }

    /// Add `other` to self.
    pub fn append(&mut self, mut other: ViewExtensions) {
        self.exts.append(&mut other.exts);
    }
}

#[cfg(windows)]
pub(crate) struct PreferAngleExt {
    pub(crate) prefer_egl: bool,
}
#[cfg(windows)]
impl PreferAngleExt {
    pub(crate) fn new(_: ApiExtensionId) -> Self {
        Self { prefer_egl: false }
    }
}
#[cfg(windows)]
impl WindowExtension for PreferAngleExt {
    fn is_init_only(&self) -> bool {
        true
    }

    fn configure(&mut self, args: &mut WindowConfigArgs) {
        if let Some(cfg) = args.config {
            match cfg.deserialize::<bool>() {
                Ok(y) => self.prefer_egl = y,
                Err(e) => tracing::error!("invalid arg for 'zng-view.prefer_angle', {e}"),
            }
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Sets renderer debug flags.
///
/// This is a test case of the extensions API.
pub(crate) struct RendererDebugExt {
    id: ApiExtensionId,
    ui: Option<String>,
}

impl RendererDebugExt {
    pub(crate) fn new(id: ApiExtensionId) -> Self {
        Self { id, ui: None }
    }
}
impl RendererExtension for RendererDebugExt {
    fn is_init_only(&self) -> bool {
        false
    }

    fn configure(&mut self, args: &mut RendererConfigArgs) {
        if let Some(cfg) = args.config.as_ref().and_then(|c| c.deserialize::<RendererDebug>().ok()) {
            args.options.debug_flags = cfg.flags;
            self.ui = Some(cfg.profiler_ui);
        }
    }

    fn renderer_inited(&mut self, args: &mut RendererInitedArgs) {
        if let Some(ui) = self.ui.take() {
            args.renderer.set_profiler_ui(&ui);
        }
    }

    fn command(&mut self, args: &mut RendererCommandArgs) -> ApiExtensionPayload {
        match args.request.deserialize::<RendererDebug>() {
            Ok(cfg) => {
                args.renderer.set_debug_flags(cfg.flags);
                args.renderer.set_profiler_ui(&cfg.profiler_ui);
                ApiExtensionPayload::empty()
            }
            Err(e) => ApiExtensionPayload::invalid_request(self.id, e),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

/// Webrender renderer debug flags and profiler UI.
#[derive(serde::Serialize, serde::Deserialize)]
struct RendererDebug {
    pub flags: DebugFlags,
    pub profiler_ui: String,
}

pub(crate) struct DisplayListExtAdapter<'a> {
    pub extensions: &'a mut Vec<(ApiExtensionId, Box<dyn RendererExtension>)>,
    pub transaction: &'a mut webrender::Transaction,
    pub renderer: &'a mut webrender::Renderer,
    pub document_id: DocumentId,
    pub api: &'a mut RenderApi,
    pub external_images: &'a mut ExternalImages,
    pub frame_id: zng_view_api::window::FrameId,
}

impl DisplayListExtension for DisplayListExtAdapter<'_> {
    fn display_list_start(&mut self, args: &mut DisplayExtensionArgs) {
        for (_, ext) in self.extensions.iter_mut() {
            ext.render_start(&mut RenderArgs {
                frame_id: self.frame_id,
                list: args.list,
                sc: args.sc,
                transaction: self.transaction,
                renderer: self.renderer,
                document_id: self.document_id,
                api: self.api,
                external_images: self.external_images,
            });
        }
    }

    fn push_display_item(&mut self, args: &mut DisplayExtensionItemArgs) {
        for (id, ext) in self.extensions.iter_mut() {
            if *id == args.extension_id {
                ext.render_push(&mut RenderItemArgs {
                    extension_id: args.extension_id,
                    payload: args.payload,
                    is_reuse: args.is_reuse,
                    list: args.list,
                    sc: args.sc,
                    transaction: self.transaction,
                    renderer: self.renderer,
                    document_id: self.document_id,
                    api: self.api,
                    external_images: self.external_images,
                });
                break;
            }
        }
    }

    fn pop_display_item(&mut self, args: &mut DisplayExtensionItemArgs) {
        for (id, ext) in self.extensions.iter_mut() {
            if *id == args.extension_id {
                ext.render_pop(&mut RenderItemArgs {
                    extension_id: args.extension_id,
                    payload: args.payload,
                    is_reuse: args.is_reuse,
                    list: args.list,
                    sc: args.sc,
                    transaction: self.transaction,
                    renderer: self.renderer,
                    document_id: self.document_id,
                    api: self.api,
                    external_images: self.external_images,
                });
                break;
            }
        }
    }

    fn display_list_end(&mut self, args: &mut DisplayExtensionArgs) {
        for (_, ext) in self.extensions.iter_mut() {
            ext.render_end(&mut RenderArgs {
                frame_id: self.frame_id,
                list: args.list,
                sc: args.sc,
                transaction: self.transaction,
                renderer: self.renderer,
                document_id: self.document_id,
                api: self.api,
                external_images: self.external_images,
            });
        }
    }

    fn update(&mut self, args: &mut DisplayExtensionUpdateArgs) {
        for (id, ext) in self.extensions.iter_mut() {
            if *id == args.extension_id {
                let mut r_args = RenderUpdateArgs {
                    extension_id: args.extension_id,
                    payload: args.payload,
                    new_frame: args.new_frame,
                    properties: args.properties,
                    document_id: self.document_id,
                    transaction: self.transaction,
                    renderer: self.renderer,
                    api: self.api,
                    external_images: self.external_images,
                };
                ext.render_update(&mut r_args);
                args.new_frame = r_args.new_frame;
                break;
            }
        }
    }
}

pub(crate) struct BlobExtensionsImgHandler(pub Vec<Box<dyn BlobExtension>>);

impl BlobImageHandler for BlobExtensionsImgHandler {
    fn create_blob_rasterizer(&mut self) -> Box<dyn AsyncBlobImageRasterizer> {
        Box::new(BlockExtensionsImgRasterizer(
            self.0.iter_mut().map(|t| t.create_blob_rasterizer()).collect(),
        ))
    }

    fn create_similar(&self) -> Box<dyn BlobImageHandler> {
        Box::new(Self(self.0.iter().map(|e| e.create_similar()).collect()))
    }

    fn prepare_resources(&mut self, services: &dyn webrender::api::BlobImageResources, requests: &[BlobImageParams]) {
        for ext in self.0.iter_mut() {
            ext.prepare_resources(&mut BlobPrepareArgs { services, requests })
        }
    }

    fn add(
        &mut self,
        key: webrender::api::BlobImageKey,
        data: std::sync::Arc<webrender::api::BlobImageData>,
        visible_rect: &webrender::api::units::DeviceIntRect,
        tile_size: webrender::api::TileSize,
    ) {
        let args = BlobAddArgs {
            key,
            data,
            visible_rect: *visible_rect,
            tile_size,
        };
        for ext in self.0.iter_mut() {
            ext.add(&args);
        }
    }

    fn update(
        &mut self,
        key: webrender::api::BlobImageKey,
        data: std::sync::Arc<webrender::api::BlobImageData>,
        visible_rect: &webrender::api::units::DeviceIntRect,
        dirty_rect: &webrender::api::units::BlobDirtyRect,
    ) {
        let args = BlobUpdateArgs {
            key,
            data,
            visible_rect: *visible_rect,
            dirty_rect: *dirty_rect,
        };
        for ext in self.0.iter_mut() {
            ext.update(&args);
        }
    }

    fn delete(&mut self, key: webrender::api::BlobImageKey) {
        for ext in self.0.iter_mut() {
            ext.delete(key);
        }
    }

    fn delete_font(&mut self, key: webrender::api::FontKey) {
        for ext in self.0.iter_mut() {
            ext.delete_font(key);
        }
    }

    fn delete_font_instance(&mut self, key: webrender::api::FontInstanceKey) {
        for ext in self.0.iter_mut() {
            ext.delete_font_instance(key);
        }
    }

    fn clear_namespace(&mut self, namespace: webrender::api::IdNamespace) {
        for ext in self.0.iter_mut() {
            ext.clear_namespace(namespace);
        }
    }

    fn enable_multithreading(&mut self, enable: bool) {
        for ext in self.0.iter_mut() {
            ext.enable_multithreading(enable);
        }
    }
}

struct BlockExtensionsImgRasterizer(Vec<Box<dyn AsyncBlobRasterizer>>);
impl AsyncBlobImageRasterizer for BlockExtensionsImgRasterizer {
    fn rasterize(
        &mut self,
        requests: &[BlobImageParams],
        low_priority: bool,
        tile_pool: &mut crate::BlobTilePool,
    ) -> Vec<(BlobImageRequest, BlobImageResult)> {
        let mut responses = vec![];
        for r in &mut self.0 {
            r.rasterize(&mut BlobRasterizerArgs {
                requests,
                low_priority,
                tile_pool,
                responses: &mut responses,
            })
        }
        responses
    }
}

/// Register a `FnOnce(&mut ViewExtensions)` closure to be called on view-process init to inject custom API extensions.
///
/// See [`ViewExtensions`] for more details.
#[macro_export]
macro_rules! view_process_extension {
    ($closure:expr) => {
        // expanded from:
        // #[linkme::distributed_slice(VIEW_EXTENSIONS)]
        // static _VIEW_EXTENSIONS: fn(&FooArgs) = foo;
        // so that users don't need to depend on linkme just to call this macro.
        #[used]
        #[cfg_attr(
            any(
                target_os = "none",
                target_os = "linux",
                target_os = "android",
                target_os = "fuchsia",
                target_os = "psp"
            ),
            unsafe(link_section = "linkme_VIEW_EXTENSIONS")
        )]
        #[cfg_attr(
            any(target_os = "macos", target_os = "ios", target_os = "tvos"),
            unsafe(link_section = "__DATA,__linkmeTbhLJz52,regular,no_dead_strip")
        )]
        #[cfg_attr(
            any(target_os = "uefi", target_os = "windows"),
            unsafe(link_section = ".linkme_VIEW_EXTENSIONS$b")
        )]
        #[cfg_attr(target_os = "illumos", unsafe(link_section = "set_linkme_VIEW_EXTENSIONS"))]
        #[cfg_attr(
            any(target_os = "freebsd", target_os = "openbsd"),
            unsafe(link_section = "linkme_VIEW_EXTENSIONS")
        )]
        #[doc(hidden)]
        static _VIEW_EXTENSIONS: fn(&mut $crate::extensions::ViewExtensions) = _view_extensions;
        #[doc(hidden)]
        fn _view_extensions(ext: &mut $crate::extensions::ViewExtensions) {
            fn view_extensions(
                ext: &mut $crate::extensions::ViewExtensions,
                handler: impl FnOnce(&mut $crate::extensions::ViewExtensions),
            ) {
                handler(ext)
            }
            view_extensions(ext, $closure)
        }
    };
}

#[doc(hidden)]
#[linkme::distributed_slice]
pub static VIEW_EXTENSIONS: [fn(&mut ViewExtensions)];
