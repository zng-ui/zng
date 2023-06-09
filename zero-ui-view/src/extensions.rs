//! Extensions API
//!
//! Extensions that run in the view-process, with internal access to things like the raw handle of windows or
//! direct access to renderers. These extensions are build on top of the view API extensions as a way to customize
//! the view-process without needing to fork it or re-implement the entire view API from scratch.
//!

use std::any::Any;

use webrender::{DebugFlags, RenderApi};
use zero_ui_view_api::{
    webrender_api::{
        AsyncBlobImageRasterizer, BlobImageHandler, BlobImageParams, BlobImageRequest, BlobImageResult, DocumentId, PipelineId,
    },
    ApiExtensionId, ApiExtensionName, ApiExtensionPayload, ApiExtensions, DisplayListExtension,
};

/// The extension API.
pub trait ViewExtension: Send + Any {
    /// Unique name and version of this extension.
    fn name(&self) -> &ApiExtensionName;

    /// Run the extension as an app level command.
    fn command(&mut self, request: ApiExtensionPayload) -> Option<ApiExtensionPayload> {
        let _ = request;
        None
    }

    /// Create a [`RendererExtension`] for a new renderer instance.
    fn renderer(&mut self) -> Option<Box<dyn RendererExtension>> {
        None
    }
}

///  Represents a view extension associated with a renderer instance.
pub trait RendererExtension: Any {
    /// Edit options for the new renderer.
    fn configure(&mut self, args: &mut RendererConfigArgs) {
        let _ = args;
    }

    /// Called just after the renderer is created.
    fn renderer_created(&mut self, args: &mut RendererCreatedArgs) {
        let _ = args;
    }

    /// If this extension can be dropped after render creation.
    fn is_config_only(&self) -> bool;

    /// Called when a command request is made for the extension and renderer (window ID).
    ///
    /// The `extension_id` is the current index of the extension, it can be used in error messages.
    fn command(&mut self, args: &mut RendererCommandArgs) -> ApiExtensionPayload {
        let _ = args;
        ApiExtensionPayload::unknown_extension(ApiExtensionId::INVALID)
    }

    /// Called when a new frame is about to begin rendering.
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

    /// Called when a new frame just finished rendering.
    fn render_end(&mut self, args: &mut RenderArgs) {
        let _ = args;
    }

    /// Called when a render-update for the extension is found.
    fn render_update(&mut self, args: &mut RenderUpdateArgs) {
        let _ = args;
    }
}

/// Arguments for [`RendererExtension::render_start`] and [`RendererExtension::render_end`].
pub struct RenderArgs<'a> {
    /// The webrender display list.
    pub list: &'a mut zero_ui_view_api::webrender_api::DisplayListBuilder,
    /// Space and clip tracker.
    pub sc: &'a mut zero_ui_view_api::SpaceAndClip,

    /// The transaction that will send the display list.
    pub transaction: &'a mut webrender::Transaction,

    /// The window or surface renderer.
    pub renderer: &'a mut webrender::Renderer,
    /// The window or surface render API.
    pub api: &'a mut RenderApi,
}

/// Arguments for [`RendererExtension::render_push`] and [`RendererExtension::render_pop`].
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
    pub list: &'a mut zero_ui_view_api::webrender_api::DisplayListBuilder,
    /// Space and clip tracker.
    pub sc: &'a mut zero_ui_view_api::SpaceAndClip,

    /// The transaction that will send the display list.
    pub transaction: &'a mut webrender::Transaction,

    /// The window or surface renderer.
    pub renderer: &'a mut webrender::Renderer,
    /// The window or surface render API.
    pub api: &'a mut RenderApi,
}

/// Arguments for [`RendererExtension::render_update`].
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
    pub properties: &'a mut zero_ui_view_api::webrender_api::DynamicProperties,

    /// The transaction that will send the properties update.
    pub transaction: &'a mut webrender::Transaction,

    /// The window or surface renderer.
    pub renderer: &'a mut webrender::Renderer,
    /// The window or surface render API.
    pub api: &'a mut RenderApi,
}

/// Represents a Webrender blob handler that can coexist with other blob handlers on the same renderer.
///
/// This API is very similar to Webrender's [`BlobImageHandler`], the only difference is that implementers
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
    fn delete(&mut self, key: zero_ui_view_api::webrender_api::BlobImageKey);

    /// Cleanup any prepared resource for the font.
    fn delete_font(&mut self, key: zero_ui_view_api::webrender_api::FontKey) {
        let _ = key;
    }

    /// Cleanup any prepared resource for the font instance.
    fn delete_font_instance(&mut self, key: zero_ui_view_api::webrender_api::FontInstanceKey) {
        let _ = key;
    }

    /// Cleanup any state related with the namespace.
    fn clear_namespace(&mut self, namespace: zero_ui_view_api::webrender_api::IdNamespace) {
        let _ = namespace;
    }

    /// Sets if multi-threading is allowed.
    ///
    /// The default is `true`, this method is only called on init if multithreading is disabled.
    fn enable_multithreading(&mut self, enable: bool);
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
pub struct BlobPrepareArgs<'a> {
    ///
    pub services: &'a dyn zero_ui_view_api::webrender_api::BlobImageResources,
    /// Requests targeting any of the blob extensions. Each extension must
    /// inspect the requests to find the ones targeting it.
    pub requests: &'a [BlobImageParams],
}

/// Arguments for [`BlobExtension::add`].
pub struct BlobAddArgs {
    /// Blob key.
    ///
    /// Blob extension must ignore this request if it did not generate this key.
    pub key: zero_ui_view_api::webrender_api::BlobImageKey,
    /// Encoded data.
    pub data: std::sync::Arc<zero_ui_view_api::webrender_api::BlobImageData>,

    ///
    pub visible_rect: zero_ui_view_api::webrender_api::units::DeviceIntRect,
    ///
    pub tile_size: zero_ui_view_api::webrender_api::TileSize,
}

/// Arguments for [`BlobExtension::update`].
pub struct BlobUpdateArgs {
    /// Blob key.
    ///
    /// Blob extension must ignore this request if it did not generate this key.
    pub key: zero_ui_view_api::webrender_api::BlobImageKey,
    /// Encoded data.
    pub data: std::sync::Arc<zero_ui_view_api::webrender_api::BlobImageData>,
    ///
    pub visible_rect: zero_ui_view_api::webrender_api::units::DeviceIntRect,
    ///
    pub dirty_rect: zero_ui_view_api::webrender_api::units::BlobDirtyRect,
}

/// Arguments for [`AsyncBlobRasterizer::rasterize`].
pub struct BlobRasterizerArgs<'a> {
    /// Rasterization requests for all rasterizers.
    ///
    /// The rasterizer must inspect the requests to find the ones targeting it.
    pub requests: &'a [BlobImageParams],
    /// Rasterization request can be schedules in a way that minimizes the risk of
    /// high priority work being enqueued behind it.
    pub low_priority: bool,

    /// Rasterization responses.
    ///
    /// Note that `requests` and `responses` are shared by all blob rasterizers, each rasterizer
    /// must inspect the requests and push responses here.
    pub responses: &'a mut Vec<(BlobImageRequest, BlobImageResult)>,
}

/// Arguments for [`RendererExtension::configure`]
pub struct RendererConfigArgs<'a> {
    /// Config payload send with the renderer creation request addressed to this extension.
    ///
    /// Note that this extension will participate in the renderer creation even if there is no config for it.
    pub config: Option<ApiExtensionPayload>,

    /// Webrender options.
    ///
    /// Note that this config is modified by the window and other extensions. Some options
    /// must not be set by extensions:
    ///
    /// * `blob_image_handler` will be set by the window to an object that aggregates
    ///    all extension blob image handlers. Add your own blob handler to `blobs` instead.
    /// * `workers` will be already set by the window, blob rasterizers may clone and use these threads.
    pub options: &'a mut webrender::WebRenderOptions,

    /// Blob extensions.
    ///
    /// Use this API instead of `blob_image_handler` in options to support multiple blob handlers.
    pub blobs: &'a mut Vec<Box<dyn BlobExtension>>,
}

/// Arguments for [`RendererExtension::renderer_created`].
pub struct RendererCreatedArgs<'a> {
    /// The new renderer.
    pub renderer: &'a mut webrender::Renderer,

    /// The API sender connected with the new renderer.
    pub api_sender: &'a webrender::RenderApiSender,

    /// The API used by the window or surface.
    pub api: &'a mut RenderApi,

    /// The document ID of the main content.
    pub document_id: DocumentId,

    /// The pipeline of the main content.
    pub pipeline_id: PipelineId,
}

/// Arguments for [`RendererExtension::command`].
pub struct RendererCommandArgs<'a> {
    /// The renderer.
    pub renderer: &'a mut webrender::Renderer,

    /// The render API used by the window or surface.
    pub api: &'a mut RenderApi,

    /// The command request.
    pub request: ApiExtensionPayload,
}

/// View extensions register.
#[derive(Default)]
pub struct ViewExtensions {
    exts: Vec<Box<dyn ViewExtension>>,
}
impl ViewExtensions {
    /// New empty.
    pub fn new() -> Self {
        Self::default()
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

    pub(crate) fn new_renderer(&mut self) -> Vec<(ApiExtensionId, Box<dyn RendererExtension>)> {
        self.exts
            .iter_mut()
            .enumerate()
            .filter_map(|(i, e)| e.renderer().map(|e| (ApiExtensionId::from_index(i), e)))
            .collect()
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
    fn is_config_only(&self) -> bool {
        false
    }

    fn configure(&mut self, args: &mut RendererConfigArgs) {
        if let Some(cfg) = args.config.as_ref().and_then(|c| c.deserialize::<RendererDebug>().ok()) {
            args.options.debug_flags = cfg.flags;
            self.ui = Some(cfg.profiler_ui);
        }
    }

    fn renderer_created(&mut self, args: &mut RendererCreatedArgs) {
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
    pub api: &'a mut RenderApi,
}

impl<'a> DisplayListExtension for DisplayListExtAdapter<'a> {
    fn display_list_start(&mut self, args: &mut zero_ui_view_api::DisplayExtensionArgs) {
        for (_, ext) in self.extensions.iter_mut() {
            ext.render_start(&mut RenderArgs {
                list: args.list,
                sc: args.sc,
                transaction: self.transaction,
                renderer: self.renderer,
                api: self.api,
            });
        }
    }

    fn push_display_item(&mut self, args: &mut zero_ui_view_api::DisplayExtensionItemArgs) {
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
                    api: self.api,
                });
                break;
            }
        }
    }

    fn pop_display_item(&mut self, args: &mut zero_ui_view_api::DisplayExtensionItemArgs) {
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
                    api: self.api,
                });
                break;
            }
        }
    }

    fn display_list_end(&mut self, args: &mut zero_ui_view_api::DisplayExtensionArgs) {
        for (_, ext) in self.extensions.iter_mut() {
            ext.render_end(&mut RenderArgs {
                list: args.list,
                sc: args.sc,
                transaction: self.transaction,
                renderer: self.renderer,
                api: self.api,
            });
        }
    }

    fn update(&mut self, args: &mut zero_ui_view_api::DisplayExtensionUpdateArgs) {
        for (id, ext) in self.extensions.iter_mut() {
            if *id == args.extension_id {
                let mut r_args = RenderUpdateArgs {
                    extension_id: args.extension_id,
                    payload: args.payload,
                    new_frame: args.new_frame,
                    properties: args.properties,
                    transaction: self.transaction,
                    renderer: self.renderer,
                    api: self.api,
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

    fn prepare_resources(&mut self, services: &dyn zero_ui_view_api::webrender_api::BlobImageResources, requests: &[BlobImageParams]) {
        for ext in self.0.iter_mut() {
            ext.prepare_resources(&mut BlobPrepareArgs { services, requests })
        }
    }

    fn add(
        &mut self,
        key: zero_ui_view_api::webrender_api::BlobImageKey,
        data: std::sync::Arc<zero_ui_view_api::webrender_api::BlobImageData>,
        visible_rect: &zero_ui_view_api::webrender_api::units::DeviceIntRect,
        tile_size: zero_ui_view_api::webrender_api::TileSize,
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
        key: zero_ui_view_api::webrender_api::BlobImageKey,
        data: std::sync::Arc<zero_ui_view_api::webrender_api::BlobImageData>,
        visible_rect: &zero_ui_view_api::webrender_api::units::DeviceIntRect,
        dirty_rect: &zero_ui_view_api::webrender_api::units::BlobDirtyRect,
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

    fn delete(&mut self, key: zero_ui_view_api::webrender_api::BlobImageKey) {
        for ext in self.0.iter_mut() {
            ext.delete(key);
        }
    }

    fn delete_font(&mut self, key: zero_ui_view_api::webrender_api::FontKey) {
        for ext in self.0.iter_mut() {
            ext.delete_font(key);
        }
    }

    fn delete_font_instance(&mut self, key: zero_ui_view_api::webrender_api::FontInstanceKey) {
        for ext in self.0.iter_mut() {
            ext.delete_font_instance(key);
        }
    }

    fn clear_namespace(&mut self, namespace: zero_ui_view_api::webrender_api::IdNamespace) {
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
    fn rasterize(&mut self, requests: &[BlobImageParams], low_priority: bool) -> Vec<(BlobImageRequest, BlobImageResult)> {
        let mut responses = vec![];
        for r in &mut self.0 {
            r.rasterize(&mut BlobRasterizerArgs {
                requests,
                low_priority,
                responses: &mut responses,
            })
        }
        responses
    }
}
