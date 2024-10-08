//! Demo view extension window.

/// App-process stuff.
pub mod app_side {
    use zng::prelude::*;
    use zng_app::view_process::VIEW_PROCESS;
    use zng_view_api::api_extension::ApiExtensionId;

    /// Get the raw-window-handle formatted to text.
    ///
    /// This sends a custom command to the view-process (implemented in `super::view_side`), the view-process
    /// uses a WindowExtension to access the raw-window-handle and format it to text as a basic example.
    pub fn window_handle(win_id: WindowId) -> Option<Txt> {
        match WINDOWS.view_window_extension::<_, super::api::Response>(
            win_id,
            self::extension_id(),
            &super::api::Request::WindowHandle { alternate: false },
        ) {
            Ok(r) => Some(r.txt),
            Err(e) => {
                tracing::error!("failed to get extension response, {e}");
                None
            }
        }
    }

    /// Get the OpenGl `GL_VERSION` text.
    pub fn gl_version(win_id: WindowId) -> Option<Txt> {
        match WINDOWS.view_window_extension::<_, super::api::Response>(win_id, self::extension_id(), &super::api::Request::GlVersion) {
            Ok(r) => Some(r.txt),
            Err(e) => {
                tracing::error!("failed to get extension response, {e}");
                None
            }
        }
    }

    pub fn extension_id() -> ApiExtensionId {
        VIEW_PROCESS
            .extension_id(super::api::extension_name())
            .ok()
            .flatten()
            .unwrap_or(ApiExtensionId::INVALID)
    }
}

/// View-process stuff, the actual extension.
pub mod view_side {
    use zng::text::formatx;
    use zng_view::extensions::WindowExtension;
    use zng_view_api::api_extension::{ApiExtensionId, ApiExtensionPayload};

    zng_view::view_process_extension!(|exts| {
        exts.window(super::api::extension_name(), CustomExtension::new);
    });

    struct CustomExtension {
        id: ApiExtensionId,
    }
    impl CustomExtension {
        fn new(id: ApiExtensionId) -> Self {
            Self { id }
        }
    }
    impl WindowExtension for CustomExtension {
        fn is_init_only(&self) -> bool {
            false
        }

        fn command(&mut self, args: &mut zng_view::extensions::WindowCommandArgs) -> ApiExtensionPayload {
            match args.request.deserialize::<super::api::Request>() {
                Ok(r) => {
                    match r {
                        super::api::Request::WindowHandle { alternate } => {
                            let h = raw_window_handle::HasWindowHandle::window_handle(args.window).unwrap();
                            ApiExtensionPayload::serialize(&super::api::Response {
                                // note that you should only use the window handle in the view-process side.
                                txt: if alternate { formatx!("{h:#?}") } else { formatx!("{h:?}") },
                            })
                            .unwrap()
                        }
                        super::api::Request::GlVersion => {
                            let version = args.context.gl().get_string(zng_view::gleam::gl::VERSION);

                            ApiExtensionPayload::serialize(&super::api::Response {
                                // note that you should only use the window handle in the view-process side.
                                txt: version.into(),
                            })
                            .unwrap()
                        }
                    }
                }
                Err(e) => ApiExtensionPayload::invalid_request(self.id, format_args!("invalid command request, {e}")),
            }
        }

        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
        fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
            self
        }
    }
}

/// Shared types.
pub mod api {
    use zng::text::Txt;
    use zng_view_api::api_extension::ApiExtensionName;

    pub fn extension_name() -> ApiExtensionName {
        ApiExtensionName::new("zng.examples.extend_renderer.get_info").unwrap()
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum Request {
        WindowHandle { alternate: bool },
        GlVersion,
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Response {
        pub txt: Txt,
    }
}
