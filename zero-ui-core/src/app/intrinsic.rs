use std::fmt;

use crate::app::*;
use crate::event::*;
use crate::service::*;
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
    pub(super) fn pre_init(
        ctx: &mut AppContext,
        is_headed: bool,
        with_renderer: bool,
        view_process_exe: Option<PathBuf>,
        device_events: bool,
    ) -> Self {
        ctx.services.register(AppProcess::new(ctx.updates.sender()));

        if is_headed {
            debug_assert!(with_renderer);

            let view_evs_sender = ctx.updates.sender();
            let view_app = ViewProcess::start(view_process_exe, device_events, false, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
            ctx.services.register(view_app);
        } else if with_renderer {
            let view_evs_sender = ctx.updates.sender();
            let renderer = ViewProcess::start(view_process_exe, false, true, move |ev| {
                let _ = view_evs_sender.send_view_event(ev);
            });
            ctx.services.register(renderer);
        }

        AppIntrinsic {
            exit_handle: EXIT_CMD.new_handle(ctx, true),
            pending_exit: None,
        }
    }

    /// Returns if exit was requested and not cancelled.
    pub(super) fn exit(&mut self, vars: &Vars) -> bool {
        if let Some(pending) = self.pending_exit.take() {
            let cancel = !pending.handle.is_stopped();
            if cancel {
                pending.response.respond(vars, ExitCancelled);
            }
            cancel
        } else {
            false
        }
    }
}
impl AppExtension for AppIntrinsic {
    fn event_preview(&mut self, ctx: &mut AppContext, update: &EventUpdate) {
        if let Some(args) = EXIT_CMD.on(update) {
            args.handle_enabled(&self.exit_handle, |_| {
                AppProcess::req(ctx.services).exit();
            });
        }
    }

    fn update(&mut self, ctx: &mut AppContext) {
        if let Some(response) = AppProcess::req(ctx.services).take_requests() {
            let args = ExitRequestedArgs::now();
            self.pending_exit = Some(PendingExit {
                handle: args.propagation().clone(),
                response,
            });
            EXIT_REQUESTED_EVENT.notify(ctx, args);
        }
    }
}

/// Service for managing the application process.
///
/// This service is registered for all apps.
#[derive(Service)]
pub struct AppProcess {
    exit_requests: Option<ResponderVar<ExitCancelled>>,
    update_sender: AppEventSender,
}
impl AppProcess {
    fn new(update_sender: AppEventSender) -> Self {
        AppProcess {
            exit_requests: None,
            update_sender,
        }
    }

    /// Register a request for process exit with code `0` in the next update.
    ///
    /// The [`EXIT_REQUESTED_EVENT`] will be raised, and if not cancelled the app process will exit.
    ///
    /// Returns a response variable that is updated once with the unit value [`ExitCancelled`]
    /// if the exit operation is cancelled.
    ///
    /// See also the [`EXIT_CMD`] that also causes an exit request.
    pub fn exit(&mut self) -> ResponseVar<ExitCancelled> {
        if let Some(r) = &self.exit_requests {
            r.response_var()
        } else {
            let (responder, response) = response_var();
            self.exit_requests = Some(responder);
            let _ = self.update_sender.send_ext_update();
            response
        }
    }

    pub(super) fn take_requests(&mut self) -> Option<ResponderVar<ExitCancelled>> {
        self.exit_requests.take()
    }
}

command! {
    /// Represents the app process [`exit`] request.
    /// 
    /// [`exit`]: AppProcess::exit
    pub static EXIT_CMD = {
        name: "Exit",
        info: "Close all windows and exit."
    };
}

/// Cancellation message of an [exit request].
///
/// [exit request]: AppProcess::exit
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitCancelled;
impl fmt::Display for ExitCancelled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "exit request cancelled")
    }
}
