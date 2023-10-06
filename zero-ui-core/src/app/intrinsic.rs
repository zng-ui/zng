use std::fmt;
use std::ops;

use crate::app::*;
use crate::event::*;
use crate::gesture::CommandShortcutExt;
use crate::shortcut;
use crate::var::*;

pub(super) struct AppIntrinsic {
    #[allow(dead_code)]
    exit_handle: CommandHandle,
    pending_exit: Option<PendingExit>,
}
struct PendingExit {
    handle: EventPropagationHandle,
    response: ResponderVar<ExitCancelled>,
}
impl AppIntrinsic {
    /// Pre-init intrinsic services and commands, must be called before extensions init.
    pub(super) fn pre_init(is_headed: bool, with_renderer: bool, view_process_exe: Option<PathBuf>, device_events: bool) -> Self {
        if is_headed {
            debug_assert!(with_renderer);

            let view_evs_sender = UPDATES.sender();
            VIEW_PROCESS.start(view_process_exe, device_events, false, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
        } else if with_renderer {
            let view_evs_sender = UPDATES.sender();
            VIEW_PROCESS.start(view_process_exe, false, true, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
        }

        AppIntrinsic {
            exit_handle: EXIT_CMD.subscribe(true),
            pending_exit: None,
        }
    }

    /// Returns if exit was requested and not cancelled.
    pub(super) fn exit(&mut self) -> bool {
        if let Some(pending) = self.pending_exit.take() {
            if pending.handle.is_stopped() {
                pending.response.respond(ExitCancelled);
                false
            } else {
                true
            }
        } else {
            false
        }
    }
}
impl AppExtension for AppIntrinsic {
    fn event_preview(&mut self, update: &mut EventUpdate) {
        if let Some(args) = EXIT_CMD.on(update) {
            args.handle_enabled(&self.exit_handle, |_| {
                APP_PROCESS.exit();
            });
        }
    }

    fn update(&mut self) {
        if let Some(response) = APP_PROCESS_SV.write().take_requests() {
            let args = ExitRequestedArgs::now();
            self.pending_exit = Some(PendingExit {
                handle: args.propagation().clone(),
                response,
            });
            EXIT_REQUESTED_EVENT.notify(args);
        }
    }
}

app_local! {
    pub(super) static APP_PROCESS_SV: AppProcessService = const {
        AppProcessService {
            exit_requests: None,
            extensions: None,
        }
    };
}

pub(super) struct AppProcessService {
    exit_requests: Option<ResponderVar<ExitCancelled>>,
    extensions: Option<Arc<AppExtensionsInfo>>,
}
impl AppProcessService {
    pub(super) fn take_requests(&mut self) -> Option<ResponderVar<ExitCancelled>> {
        self.exit_requests.take()
    }

    fn exit(&mut self) -> ResponseVar<ExitCancelled> {
        if let Some(r) = &self.exit_requests {
            r.response_var()
        } else {
            let (responder, response) = response_var();
            self.exit_requests = Some(responder);
            UPDATES.update(None);
            response
        }
    }

    pub(super) fn extensions(&self) -> Arc<AppExtensionsInfo> {
        self.extensions
            .clone()
            .unwrap_or_else(|| Arc::new(AppExtensionsInfo { infos: vec![] }))
    }

    pub(super) fn set_extensions(&mut self, info: AppExtensionsInfo) {
        self.extensions = Some(Arc::new(info));
    }
}

/// Info about an app-extension.
///
/// See [`App::extensions`] for more details.
///
/// [`App::extensions`]: crate::app::App::extensions
#[derive(Clone, Copy)]
pub struct AppExtensionInfo {
    /// Extension type ID.
    pub type_id: TypeId,
    /// Extension type name.
    pub type_name: &'static str,
}
impl PartialEq for AppExtensionInfo {
    fn eq(&self, other: &Self) -> bool {
        self.type_id == other.type_id
    }
}
impl fmt::Debug for AppExtensionInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.type_name)
    }
}
impl Eq for AppExtensionInfo {}
impl AppExtensionInfo {
    /// New info for `E`.
    pub fn new<E: AppExtension>() -> Self {
        Self {
            type_id: TypeId::of::<E>(),
            type_name: type_name::<E>(),
        }
    }
}

/// List of app-extensions that are part of an app.
#[derive(Clone, PartialEq)]
pub struct AppExtensionsInfo {
    infos: Vec<AppExtensionInfo>,
}
impl fmt::Debug for AppExtensionsInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(&self.infos).finish()
    }
}
impl AppExtensionsInfo {
    pub(super) fn start() -> Self {
        Self { infos: vec![] }
    }

    /// Push the extension info.
    pub fn push<E: AppExtension>(&mut self) {
        let info = AppExtensionInfo::new::<E>();
        assert!(!self.contains::<E>(), "app-extension `{info:?}` is already in the list");
        self.infos.push(info);
    }

    /// Gets if the extension `E` is in the list.
    pub fn contains<E: AppExtension>(&self) -> bool {
        self.contains_info(AppExtensionInfo::new::<E>())
    }

    /// Gets i the extension is in the list.
    pub fn contains_info(&self, info: AppExtensionInfo) -> bool {
        self.infos.iter().any(|e| e.type_id == info.type_id)
    }

    /// Panics if the extension `E` is not present.
    #[track_caller]
    pub fn require<E: AppExtension>(&self) {
        let info = AppExtensionInfo::new::<E>();
        assert!(self.contains_info(info), "app-extension `{info:?}` is required");
    }
}
impl ops::Deref for AppExtensionsInfo {
    type Target = [AppExtensionInfo];

    fn deref(&self) -> &Self::Target {
        &self.infos
    }
}

/// Service for managing the application process.
///
/// This service is available in all apps.
#[allow(non_camel_case_types)]
pub struct APP_PROCESS;
impl APP_PROCESS {
    /// Register a request for process exit with code `0` in the next update.
    ///
    /// The [`EXIT_REQUESTED_EVENT`] will be raised, and if not cancelled the app process will exit.
    ///
    /// Returns a response variable that is updated once with the unit value [`ExitCancelled`]
    /// if the exit operation is cancelled.
    ///
    /// See also the [`EXIT_CMD`] that also causes an exit request.
    pub fn exit(&self) -> ResponseVar<ExitCancelled> {
        APP_PROCESS_SV.write().exit()
    }
}

command! {
    /// Represents the app process [`exit`] request.
    ///
    /// [`exit`]: APP_PROCESS::exit
    pub static EXIT_CMD = {
        name: "Exit",
        info: "Close all windows and exit.",
        shortcut: shortcut!(Exit),
    };
}

/// Cancellation message of an [exit request].
///
/// [exit request]: APP_PROCESS::exit
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitCancelled;
impl fmt::Display for ExitCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exit request cancelled")
    }
}
