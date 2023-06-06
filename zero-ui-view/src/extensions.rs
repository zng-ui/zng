//! Extensions API
//!
//! Extensions that run in the view-process, with internal access to things like the raw handle of windows or
//! direct access to renderers. These extensions are build on top of the view API extensions as a way to customize
//! the view-process without needing to fork it or re-implement the entire view API from scratch.
//!

use std::any::Any;

use webrender::{DebugFlags, RenderApi};
use zero_ui_view_api::{ApiExtensionName, ApiExtensions, ExtensionPayload};

/// The extension API.
pub trait ViewExtension: Send + Any {
    /// Unique name and version of this extension.
    fn name(&self) -> &ApiExtensionName;

    /// Run the extension as an app level command.
    fn command(&mut self, request: ExtensionPayload) -> Option<ExtensionPayload> {
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
    ///
    /// The `cfg` is the raw config send with the renderer creation request addressing this extension. Note
    /// that this extension will participate in the renderer creation even if there is no config for it.
    fn configure(&mut self, cfg: Option<ExtensionPayload>, opts: &mut webrender::WebRenderOptions) {
        let _ = (cfg, opts);
    }

    /// Called just after the renderer is created.
    fn renderer_created(&mut self, renderer: &mut webrender::Renderer, api_sender: &webrender::RenderApiSender) {
        let _ = (renderer, api_sender);
    }

    /// If this extension can be dropped after render creation.
    fn is_config_only(&self) -> bool;

    /// Called when a command request is made for the extension and renderer (window ID).
    ///
    /// The `extension_key` is the current index of the extension, it can be used in error messages.
    fn command(
        &mut self,
        renderer: &mut webrender::Renderer,
        render_api: &RenderApi,
        request: ExtensionPayload,
        extension_key: usize,
    ) -> ExtensionPayload {
        let _ = (renderer, render_api, request);
        ExtensionPayload::unknown_extension(extension_key)
    }

    /// Called when a new frame is about to begin rendering.
    fn begin_render(&mut self) {}

    /// Called when a new frame just finished rendering.
    fn finish_render(&mut self) {}

    /// Called when a display item push for the extension is found.
    fn display_item_push(&mut self, payload: &mut ExtensionPayload, wr_list: &mut zero_ui_view_api::webrender_api::DisplayListBuilder) {
        let _ = (payload, wr_list);
    }

    /// Called when a display item pop for the extension is found.
    fn display_item_pop(&mut self) {}
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

    /// Register an extension.
    ///
    /// # Panics
    ///
    /// Panics if the name is already registered.
    pub fn register(&mut self, ext: Box<dyn ViewExtension>) -> &mut Self {
        if self.is_registered(ext.name()) {
            panic!("extension `{:?}` is already registered", ext.name());
        }
        self.exts.push(ext);
        self
    }

    /// Returns `true` is an extension of the same name is already registered.
    pub fn is_registered(&self, name: &ApiExtensionName) -> bool {
        self.exts.iter().any(|e| e.name() == name)
    }

    /// Register a command extension with custom encoded messages.
    pub fn command_raw(
        &mut self,
        name: impl Into<ApiExtensionName>,
        handler: impl FnMut(ExtensionPayload) -> ExtensionPayload + Send + 'static,
    ) -> &mut Self {
        struct CommandExt<F>(ApiExtensionName, F);
        impl<F: FnMut(ExtensionPayload) -> ExtensionPayload + Send + 'static> ViewExtension for CommandExt<F> {
            fn name(&self) -> &ApiExtensionName {
                &self.0
            }
            fn command(&mut self, request: ExtensionPayload) -> Option<ExtensionPayload> {
                Some((self.1)(request))
            }
        }

        self.register(Box::new(CommandExt(name.into(), handler)));
        self
    }

    /// Register a command extension.
    pub fn command<I: serde::de::DeserializeOwned, O: serde::Serialize>(
        &mut self,
        name: impl Into<ApiExtensionName>,
        mut handler: impl FnMut(I) -> O + Send + 'static,
    ) -> &mut Self {
        self.command_raw(name, move |i| match i.deserialize::<I>() {
            Ok(i) => {
                let o = handler(i);
                ExtensionPayload::serialize(&o).unwrap()
            }
            Err(e) => ExtensionPayload::invalid_request(usize::MAX, e),
        })
    }

    /// Register a renderer extension.
    pub fn renderer<E: RendererExtension>(
        &mut self,
        name: impl Into<ApiExtensionName>,
        new: impl FnMut() -> E + Send + 'static,
    ) -> &mut Self {
        struct RendererExt<F>(ApiExtensionName, F);
        impl<E, F> ViewExtension for RendererExt<F>
        where
            E: RendererExtension,
            F: FnMut() -> E + Send + 'static,
        {
            fn name(&self) -> &ApiExtensionName {
                &self.0
            }

            fn renderer(&mut self) -> Option<Box<dyn RendererExtension>> {
                Some(Box::new((self.1)()))
            }
        }
        self.register(Box::new(RendererExt(name.into(), new)));
        self
    }

    pub(crate) fn api_extensions(&self) -> ApiExtensions {
        let mut r = ApiExtensions::new();
        for ext in &self.exts {
            r.insert(ext.name().clone()).unwrap();
        }
        r
    }

    pub(crate) fn call_command(&mut self, key: usize, request: ExtensionPayload) -> ExtensionPayload {
        if key >= self.exts.len() {
            ExtensionPayload::unknown_extension(key)
        } else if let Some(r) = self.exts[key].command(request) {
            r
        } else {
            ExtensionPayload::unknown_extension(key)
        }
    }

    pub(crate) fn new_renderer(&mut self) -> Vec<(usize, Box<dyn RendererExtension>)> {
        self.exts
            .iter_mut()
            .enumerate()
            .filter_map(|(i, e)| e.renderer().map(|e| (i, e)))
            .collect()
    }
}

/// Sets renderer debug flags.
///
/// This is a test case of the extensions API.
#[derive(Default)]
pub(crate) struct RendererDebugExt {
    ui: Option<String>,
}
impl RendererExtension for RendererDebugExt {
    fn is_config_only(&self) -> bool {
        false
    }

    fn configure(&mut self, cfg: Option<ExtensionPayload>, opts: &mut webrender::WebRenderOptions) {
        if let Some(cfg) = cfg.and_then(|c| c.deserialize::<RendererDebug>().ok()) {
            opts.debug_flags = cfg.flags;
            self.ui = Some(cfg.profiler_ui);
        }
    }

    fn renderer_created(&mut self, renderer: &mut webrender::Renderer, _: &webrender::RenderApiSender) {
        if let Some(ui) = self.ui.take() {
            renderer.set_profiler_ui(&ui);
        }
    }

    fn command(
        &mut self,
        renderer: &mut webrender::Renderer,
        _: &RenderApi,
        request: ExtensionPayload,
        extension_key: usize,
    ) -> ExtensionPayload {
        match request.deserialize::<RendererDebug>() {
            Ok(cfg) => {
                renderer.set_debug_flags(cfg.flags);
                renderer.set_profiler_ui(&cfg.profiler_ui);
                ExtensionPayload::empty()
            }
            Err(e) => ExtensionPayload::invalid_request(extension_key, e),
        }
    }
}

/// Webrender renderer debug flags and profiler UI.
#[derive(serde::Serialize, serde::Deserialize)]
struct RendererDebug {
    pub flags: DebugFlags,
    pub profiler_ui: String,
}
