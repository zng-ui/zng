//! Extensions API
//!
//! Extensions that run in the view-process, with internal access to things like the raw handle of windows or
//! direct access to renderers. These extensions are build on top of the view API extensions as a way to customize
//! the view-process without needing to fork it or re-implement the entire view API from scratch.
//!

use std::any::Any;

use webrender::{DebugFlags, RenderApi};
use zero_ui_view_api::{
    webrender_api::{DocumentId, PipelineId},
    ApiExtensionId, ApiExtensionName, ApiExtensionPayload, ApiExtensions, DisplayExtensionItemArgs, DisplayExtensionUpdateArgs,
    DisplayListExtension,
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
    fn display_list_start(&mut self, args: &mut zero_ui_view_api::DisplayExtensionArgs) {
        let _ = args;
    }

    /// Called when a new frame just finished rendering.
    fn display_list_end(&mut self, args: &mut zero_ui_view_api::DisplayExtensionArgs) {
        let _ = args;
    }

    /// Called when a display item push for the extension is found.
    fn display_item_push(&mut self, args: &mut DisplayExtensionItemArgs) {
        let _ = args;
    }

    /// Called when a display item pop for the extension is found.
    fn display_item_pop(&mut self, args: &mut DisplayExtensionItemArgs) {
        let _ = args;
    }

    /// Called when a render-update for the extension is found.
    fn render_update(&mut self, args: &mut DisplayExtensionUpdateArgs) {
        let _ = args;
    }
}

/// Arguments for [`RendererExtension::configure`]
pub struct RendererConfigArgs<'a> {
    /// Config payload send with the renderer creation request addressed to this extension.
    ///
    /// Note that this extension will participate in the renderer creation even if there is no config for it.
    pub config: Option<ApiExtensionPayload>,

    /// Webrender options.
    ///
    /// Note that this config is modified by the base implementation and other extensions. Some options
    /// must not be changed, in particular the `blob_image_handler` will be set by the base implementation
    /// to an object that aggregates all extension blob image handlers.
    pub options: &'a mut webrender::WebRenderOptions,
}

/// Arguments for [`RendererExtension::renderer_created`].
pub struct RendererCreatedArgs<'a> {
    /// The new renderer.
    pub renderer: &'a mut webrender::Renderer,

    /// The API sender connected with the new renderer.
    pub api_sender: &'a webrender::RenderApiSender,

    /// The API used by the window or surface.
    pub api: &'a RenderApi,

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
    pub api: &'a RenderApi,

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

pub(crate) struct DisplayListExtAdapter<'a>(pub &'a mut Vec<(ApiExtensionId, Box<dyn RendererExtension>)>);

impl<'a> DisplayListExtension for DisplayListExtAdapter<'a> {
    fn display_list_start(&mut self, args: &mut zero_ui_view_api::DisplayExtensionArgs) {
        for (_, ext) in self.0.iter_mut() {
            ext.display_list_start(args);
        }
    }

    fn push_display_item(&mut self, args: &mut zero_ui_view_api::DisplayExtensionItemArgs) {
        for (id, ext) in self.0.iter_mut() {
            if *id == args.extension_id {
                ext.display_item_push(args);
                break;
            }
        }
    }

    fn pop_display_item(&mut self, args: &mut zero_ui_view_api::DisplayExtensionItemArgs) {
        for (id, ext) in self.0.iter_mut() {
            if *id == args.extension_id {
                ext.display_item_pop(args);
                break;
            }
        }
    }

    fn display_list_end(&mut self, args: &mut zero_ui_view_api::DisplayExtensionArgs) {
        for (_, ext) in self.0.iter_mut() {
            ext.display_list_end(args);
        }
    }

    fn update(&mut self, args: &mut zero_ui_view_api::DisplayExtensionUpdateArgs) {
        for (id, ext) in self.0.iter_mut() {
            if *id == args.extension_id {
                ext.render_update(args);
                break;
            }
        }
    }
}
