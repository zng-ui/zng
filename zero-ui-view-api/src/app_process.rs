use std::{
    panic,
    path::{Path, PathBuf},
    thread::{self, JoinHandle},
    time::Instant,
};

#[cfg(feature = "ipc")]
use std::time::Duration;

use crate::{ipc, AnyResult, Event, Request, Respawned, Response, ViewConfig, ViewProcessGen, VpResult};

/// The listener returns the closure on join for reuse in respawn.
type EventListenerJoin = JoinHandle<Box<dyn FnMut(Event) + Send>>;

#[cfg(feature = "ipc")]
type DuctHandle = duct::Handle;
#[cfg(not(feature = "ipc"))]
struct DuctHandle;

pub(crate) const SERVER_NAME_VAR: &str = "ZERO_UI_WR_SERVER";
pub(crate) const MODE_VAR: &str = "ZERO_UI_WR_MODE";

/// View Process controller, used in the App Process.
///
/// # Shutdown
///
/// The View Process is [killed] when the controller is dropped, if the app is running in same process mode
/// then the current process [exits] with code 0 on drop.
///
/// [killed]: std::process::Child::kill
/// [exits]: std::process::exit
#[cfg_attr(not(feature = "ipc"), allow(unused))]
pub struct Controller {
    process: Option<DuctHandle>,
    generation: ViewProcessGen,
    view_process_exe: PathBuf,
    request_sender: ipc::RequestSender,
    response_receiver: ipc::ResponseReceiver,
    event_listener: Option<EventListenerJoin>,
    headless: bool,
    same_process: bool,
    device_events: bool,
    last_respawn: Option<Instant>,
    fast_respawn_count: u8,
}
impl Controller {
    /// Start with a custom view process.
    ///
    /// The `view_process_exe` must be an executable that starts a view server, if not set
    /// the [`current_exe`] is used. Note that the [`VERSION`] of this crate must match in both executables.
    ///
    /// The `on_event` closure is called in another thread every time the app receives an event.
    ///
    /// # Tests
    ///
    /// The [`current_exe`] cannot be used in tests, you should set an external view-process executable. Unfortunately there
    /// is no way to check if `start` was called in a test so we cannot provide an error message for this.
    /// If the test is hanging in debug builds or has a timeout error in release builds this is probably the reason.
    ///
    /// [`current_exe`]: std::env::current_exe
    /// [`VERSION`]: crate::VERSION
    pub fn start<F>(view_process_exe: Option<PathBuf>, device_events: bool, headless: bool, mut on_event: F) -> Self
    where
        F: FnMut(Event) + Send + 'static,
    {
        if ViewConfig::from_env().is_some() {
            panic!("cannot start Controller in process configured to be view-process");
        }

        let view_process_exe = view_process_exe.unwrap_or_else(|| {
            std::env::current_exe().expect("failed to get the current exetuable, consider using an external view-process exe")
        });

        let (process, request_sender, response_receiver, mut event_receiver) =
            Self::spawn_view_process(&view_process_exe, headless).expect("failed to spawn or connect to view-process");

        let ev = thread::spawn(move || {
            while let Ok(ev) = event_receiver.recv() {
                on_event(ev);
            }
            on_event(Event::Disconnected(1));

            // return to reuse in respawn.
            let t: Box<dyn FnMut(Event) + Send> = Box::new(on_event);
            t
        });

        let mut c = Controller {
            same_process: process.is_none(),
            process,
            view_process_exe,
            request_sender,
            response_receiver,
            event_listener: Some(ev),
            headless,
            device_events,
            generation: 1,
            last_respawn: None,
            fast_respawn_count: 0,
        };

        if let Err(Respawned) = c.try_startup() {
            panic!("respawn on startup"); // TODO recover from this
        }

        c
    }
    fn try_startup(&mut self) -> VpResult<()> {
        if crate::VERSION != self.api_version()? {
            panic!("app-process and view-process must be build using the same exact version of zero-ui-vp");
        }
        self.startup(self.generation, self.device_events, self.headless)?;
        Ok(())
    }

    /// View-process generation.
    pub fn generation(&self) -> ViewProcessGen {
        self.generation
    }

    /// If is running in headless mode.
    #[inline]
    pub fn headless(&self) -> bool {
        self.headless
    }

    /// If device events are enabled.
    #[inline]
    pub fn device_events(&self) -> bool {
        self.device_events
    }

    /// If is running both view and app in the same process.
    #[inline]
    pub fn same_process(&self) -> bool {
        self.same_process
    }

    fn try_talk(&mut self, req: Request) -> ipc::IpcResult<Response> {
        self.request_sender.send(req)?;
        self.response_receiver.recv()
    }
    pub(crate) fn talk(&mut self, req: Request) -> VpResult<Response> {
        debug_assert!(req.expect_response());

        match self.try_talk(req) {
            Ok(r) => Ok(r),
            Err(ipc::Disconnected) => {
                self.handle_disconnect(self.generation);
                Err(Respawned)
            }
        }
    }

    pub(crate) fn command(&mut self, req: Request) -> VpResult<()> {
        debug_assert!(!req.expect_response());

        match self.request_sender.send(req) {
            Ok(_) => Ok(()),
            Err(ipc::Disconnected) => {
                self.handle_disconnect(self.generation);
                Err(Respawned)
            }
        }
    }

    fn spawn_view_process(
        view_process_exe: &Path,
        headless: bool,
    ) -> AnyResult<(Option<DuctHandle>, ipc::RequestSender, ipc::ResponseReceiver, ipc::EventReceiver)> {
        let init = ipc::AppInit::new();

        // create process and spawn it, unless is running in same process mode.
        let process = if ViewConfig::is_awaiting_same_process() {
            ViewConfig::set_same_process(ViewConfig {
                server_name: init.name().to_owned(),
                headless,
            });
            None
        } else {
            #[cfg(not(feature = "ipc"))]
            {
                let _ = view_process_exe;
                panic!("expected only same_process mode with `ipc` feature disabled");
            }

            #[cfg(feature = "ipc")]
            {
                let process = duct::cmd!(view_process_exe)
                    .env(SERVER_NAME_VAR, init.name())
                    .env(MODE_VAR, if headless { "headless" } else { "headed" })
                    .env("RUST_BACKTRACE", "full")
                    .stdout_capture()
                    .stderr_capture()
                    .unchecked()
                    .start()?;
                Some(process)
            }
        };

        let (req, rsp, ev) = match init.connect() {
            Ok(r) => r,
            Err(e) => {
                #[cfg(feature = "ipc")]
                if let Some(p) = process {
                    if let Err(ke) = p.kill() {
                        tracing::error!(
                            "failed to kill new view-process after failing to connect to it\n connection error: {:?}\n kill error: {:?}",
                            e,
                            ke
                        );
                    }
                }
                return Err(e);
            }
        };

        Ok((process, req, rsp, ev))
    }

    /// Handle an [`Event::Disconnected`].
    ///
    /// The `gen` parameter is the generation provided by the event. It is used to determinate if the disconnect has
    /// not been handled already.
    ///
    /// Tries to cleanup the old view-process and start a new one, if all is successful an [`Event::Respawned`] is send.
    ///
    /// The old view-process exit code and std output is logged using the `vp_respawn` target.
    ///
    /// Exits the current process with code `1` if the view-process was killed by the user. In Windows this is if
    /// the view-process exit code is `1` and in Unix if there is no exit code (killed by signal).
    ///
    /// # Panics
    ///
    /// If the last five respawns happened all within 500ms of the previous respawn.
    ///
    /// If the an error happens three times when trying to spawn the new view-process.
    ///
    /// If another disconnect happens during the view-process startup dialog.
    pub fn handle_disconnect(&mut self, gen: ViewProcessGen) {
        if gen == self.generation {
            #[cfg(not(feature = "ipc"))]
            {
                tracing::error!(target: "vp_respawn", "cannot recover in same_process mode (no ipc)");
            }

            #[cfg(feature = "ipc")]
            {
                self.respawn_impl(true)
            }
        }
    }

    /// Reopen the view-process, causing an [`Event::Respawned`].
    ///
    /// This is similar to [`handle_disconnect`] but the current process does not
    /// exit depending on the view-process exit code.
    ///
    /// [`handle_disconnect`]: Controller::handle_disconnect
    pub fn respawn(&mut self) {
        #[cfg(not(feature = "ipc"))]
        {
            tracing::error!(target: "vp_respawn", "cannot recover in same_process mode (no ipc)");
        }

        #[cfg(feature = "ipc")]
        self.respawn_impl(false);
    }
    #[cfg(feature = "ipc")]
    fn respawn_impl(&mut self, is_crash: bool) {
        let process = if let Some(p) = self.process.take() {
            p
        } else {
            if self.same_process {
                tracing::error!(target: "vp_respawn", "cannot recover in same_process mode");
            }
            return;
        };
        if is_crash {
            tracing::error!(target: "vp_respawn", "channel disconnect, will try respawn");
        }

        let t = Instant::now();
        if let Some(last_respawn) = self.last_respawn {
            if t - last_respawn < Duration::from_secs(60) {
                self.fast_respawn_count += 1;
                if self.fast_respawn_count == 2 {
                    panic!("disconnect respawn happened 2 times less than 1 minute apart");
                }
            } else {
                self.fast_respawn_count = 0;
            }
        }
        self.last_respawn = Some(t);

        // try exit
        let mut killed_by_us = false;
        if !is_crash {
            let _ = process.kill();
            killed_by_us = true;
        } else if !matches!(process.try_wait(), Ok(Some(_))) {
            // if not exited, give the process 300ms to close with the preferred exit code.
            thread::sleep(Duration::from_millis(300));

            if !matches!(process.try_wait(), Ok(Some(_))) {
                // if still not exited, kill it.
                killed_by_us = true;
                let _ = process.kill();
            }
        }

        let code_and_output = match process.into_output() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::error!(target: "vp_respawn", "view-process could not be heaped, will abandon running, {:?}", e);
                None
            }
        };

        // try print stdout/err and exit code.
        if let Some(c) = code_and_output {
            tracing::info!(target: "vp_respawn", "view-process reaped");

            let code = c.status.code();

            if !killed_by_us {
                // check if user killed the view-process, in this case we exit too.

                #[cfg(windows)]
                if code == Some(1) {
                    tracing::warn!(target: "vp_respawn", "view-process exit code is `1`, probably killed by the system, \
                                        will exit app-process with the same code");
                    std::process::exit(1);
                }

                #[cfg(unix)]
                if code == None {
                    tracing::warn!(target: "vp_respawn", "view-process exited by signal, probably killed by the user, \
                                        will exit app-process too");
                    std::process::exit(1);
                }
            }

            let code = code.unwrap();

            if !killed_by_us {
                tracing::error!(target: "vp_respawn", "view-process exit_code: 0x{:x}", code);
            }

            match String::from_utf8(c.stderr) {
                Ok(s) => {
                    if !s.is_empty() {
                        tracing::error!(target: "vp_respawn", "view-process stderr:\n```stderr\n{}\n```", s)
                    }
                }
                Err(e) => tracing::error!(target: "vp_respawn", "failed to read view-process stderr: {}", e),
            }

            match String::from_utf8(c.stdout) {
                Ok(s) => {
                    if !s.is_empty() {
                        tracing::info!(target: "vp_respawn", "view-process stdout:\n```stdout\n{}\n```", s)
                    }
                }
                Err(e) => tracing::error!(target: "vp_respawn", "failed to read view-process stdout: {}", e),
            }
        } else {
            tracing::error!(target: "vp_respawn", "failed to reap view-process, will abandon it running and spawn a new one");
        }

        // recover event listener closure (in a box).
        let mut on_event = match self.event_listener.take().unwrap().join() {
            Ok(fn_) => fn_,
            Err(p) => panic::resume_unwind(p),
        };

        // respawn
        let mut retries = 3;
        let (new_process, request, response, mut event) = loop {
            match Self::spawn_view_process(&self.view_process_exe, self.headless) {
                Ok(r) => break r,
                Err(e) => {
                    tracing::error!(target: "vp_respawn",  "failed to respawn, {:?}", e);
                    retries -= 1;
                    if retries == 0 {
                        panic!("failed to respawn `view-process` after 3 retries");
                    }
                    tracing::info!(target: "vp_respawn", "retrying respawn");
                }
            }
        };

        // update connections
        self.process = new_process;
        self.request_sender = request;
        self.response_receiver = response;

        let mut next_id = self.generation.wrapping_add(1);
        if next_id == 0 {
            next_id = 1;
        }
        self.generation = next_id;

        if let Err(Respawned) = self.try_startup() {
            panic!("respawn on respawn startup");
        }

        let ev = thread::spawn(move || {
            // notify a respawn.
            on_event(Event::Respawned(next_id));
            while let Ok(ev) = event.recv() {
                on_event(ev);
            }
            on_event(Event::Disconnected(next_id));

            on_event
        });
        self.event_listener = Some(ev);
    }
}
impl Drop for Controller {
    /// Kills the View Process, unless it is running in the same process.
    fn drop(&mut self) {
        let _ = self.exit();
        #[cfg(feature = "ipc")]
        if let Some(process) = self.process.take() {
            let _ = process.kill();
        }
    }
}
