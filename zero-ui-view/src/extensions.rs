//! Extensions API
//!
//! Extensions that run in the view-process, with internal access to things like the raw handle of windows or
//! direct access to renderers. These extensions are build on top of the view API extensions as a way to customize
//! the view-process without needing to fork it or re-implement the entire view API from scratch.
//!

use std::any::Any;

use zero_ui_view_api::{ApiExtensionName, ApiExtensions, ExtensionPayload};

/// The extension API.
pub trait ViewExtension: Send + Any {
    /// Unique name and version of this extension.
    fn name(&self) -> &ApiExtensionName;

    /// Run the extension as a command.
    fn command(&mut self, request: ExtensionPayload) -> Option<ExtensionPayload> {
        let _ = request;
        None
    }
}

/// View extensions register.
#[derive(Default)]
pub struct ViewExtensions {
    ext: Vec<Box<dyn ViewExtension>>,
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
        self.ext.push(ext);
        self
    }

    /// Returns `true` is an extension of the same name is already registered.
    pub fn is_registered(&self, name: &ApiExtensionName) -> bool {
        self.ext.iter().any(|e| e.name() == name)
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

    pub(crate) fn api_extensions(&self) -> ApiExtensions {
        let mut r = ApiExtensions::new();
        for ext in &self.ext {
            r.insert(ext.name().clone()).unwrap();
        }
        r
    }

    pub(crate) fn call_command(&mut self, key: usize, request: ExtensionPayload) -> ExtensionPayload {
        if key >= self.ext.len() {
            ExtensionPayload::unknown_extension(key)
        } else if let Some(r) = self.ext[key].command(request) {
            r
        } else {
            ExtensionPayload::unknown_extension(key)
        }
    }

    pub(crate) fn contains(&self, extension_key: usize) -> bool {
        self.ext.len() > extension_key
    }
}
