use std::{
    collections::HashMap,
    panic,
    path::{Path, PathBuf},
    sync::Arc,
    thread::{self, JoinHandle},
    time::Instant,
};

#[cfg(ipc)]
use std::time::Duration;

use parking_lot::Mutex;
use zng_txt::Txt;

use crate::{
    AnyResult, Event, Request, Response, ViewConfig, ViewProcessGen, VpResult,
    ipc::{self, EventReceiver},
};

/// The listener returns the closure on join for reuse in respawn.
type EventListenerJoin = JoinHandle<Box<dyn FnMut(Event) + Send>>;

pub(crate) const VIEW_VERSION: &str = "ZNG_VIEW_VERSION";
pub(crate) const VIEW_SERVER: &str = "ZNG_VIEW_SERVER";
pub(crate) const VIEW_MODE: &str = "ZNG_VIEW_MODE";

#[derive(Clone, Copy)]
enum ViewState {
    NotRunning,
    RunningAndConnected,
    Suspended,
}

/// View Process controller, used in the App Process.
///
/// # Exit
///
/// The View Process is [killed] when the controller is dropped, if the app is running in same process mode
/// then the current process [exits] with code 0 on drop.
///
/// [killed]: std::process::Child::kill
/// [exits]: std::process::exit
#[cfg_attr(not(ipc), allow(unused))]
pub struct Controller {
    process: Arc<Mutex<Option<std::process::Child>>>,
    view_state: ViewState,
    generation: ViewProcessGen,
    is_respawn: bool,
    view_process_exe: PathBuf,
    view_process_env: HashMap<Txt, Txt>,
    request_sender: ipc::RequestSender,
    response_receiver: ipc::ResponseReceiver,
    event_listener: Option<EventListenerJoin>,
    headless: bool,
    same_process: bool,
    last_respawn: Option<Instant>,
    fast_respawn_count: u8,
}
#[cfg(test)]
fn _assert_sync(x: Controller) -> impl Send + Sync {
    x
}
impl Controller {
    /// Start with a custom view process.
    ///
    /// The `view_process_exe` must be an executable that starts a view server.
    /// Note that the [`VERSION`] of this crate must match in both executables.
    ///
    /// The `view_process_env` can be set to any env var needed to start the view-process. Note that if `view_process_exe`
    /// is the current executable this most likely need set `zng_env::PROCESS_MAIN`.
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
    pub fn start<F>(view_process_exe: PathBuf, view_process_env: HashMap<Txt, Txt>, headless: bool, on_event: F) -> Self
    where
        F: FnMut(Event) + Send + 'static,
    {
        Self::start_impl(view_process_exe, view_process_env, headless, Box::new(on_event))
    }
    fn start_impl(
        view_process_exe: PathBuf,
        view_process_env: HashMap<Txt, Txt>,
        headless: bool,
        on_event: Box<dyn FnMut(Event) + Send>,
    ) -> Self {
        if ViewConfig::from_env().is_some() {
            panic!("cannot start Controller in process configured to be view-process");
        }

        let (process, request_sender, response_receiver, event_receiver) =
            Self::spawn_view_process(&view_process_exe, &view_process_env, headless).expect("failed to spawn or connect to view-process");
        let same_process = process.is_none();
        let process = Arc::new(Mutex::new(process));
        let ev = if same_process {
            Self::spawn_same_process_listener(on_event, event_receiver)
        } else {
            Self::spawn_other_process_listener(on_event, event_receiver, process.clone())
        };

        let mut c = Controller {
            same_process,
            view_state: ViewState::NotRunning,
            process,
            view_process_exe,
            view_process_env,
            request_sender,
            response_receiver,
            event_listener: Some(ev),
            headless,
            generation: ViewProcessGen::first(),
            is_respawn: false,
            last_respawn: None,
            fast_respawn_count: 0,
        };

        if let Err(ipc::ViewChannelError::Disconnected) = c.try_init() {
            panic!("respawn on init");
        }

        c
    }
    fn spawn_same_process_listener(
        mut on_event: Box<dyn FnMut(Event) + Send>,
        mut event_receiver: EventReceiver,
    ) -> std::thread::JoinHandle<Box<dyn FnMut(Event) + Send>> {
        thread::spawn(move || {
            while let Ok(ev) = event_receiver.recv() {
                on_event(ev);
            }
            on_event(Event::Disconnected(ViewProcessGen::first()));

            // return to reuse in respawn.
            on_event
        })
    }
    fn spawn_other_process_listener(
        mut on_event: Box<dyn FnMut(Event) + Send>,
        mut event_receiver: EventReceiver,
        process: Arc<Mutex<Option<std::process::Child>>>,
    ) -> std::thread::JoinHandle<Box<dyn FnMut(Event) + Send>> {
        // ipc-channel sometimes does not signal disconnect when the view-process dies
        thread::spawn(move || {
            let ping_time = Duration::from_secs(1);
            while let Ok(maybe) = event_receiver.try_recv_timeout(ping_time) {
                match maybe {
                    Some(ev) => on_event(ev),
                    None => {
                        if let Some(p) = &mut *process.lock() {
                            match p.try_wait() {
                                Ok(c) => {
                                    if c.is_some() {
                                        // view-process died
                                        break;
                                    }
                                }
                                Err(e) => {
                                    if e.kind() != std::io::ErrorKind::Interrupted {
                                        // signal disconnected to trigger a respawn
                                        break;
                                    }
                                }
                            }
                        } else {
                            // respawning already
                            break;
                        }
                    }
                }
            }
            on_event(Event::Disconnected(ViewProcessGen::first()));

            // return to reuse in respawn.
            on_event
        })
    }

    fn try_init(&mut self) -> VpResult<()> {
        self.init(self.generation, self.is_respawn, self.headless)?;
        Ok(())
    }

    /// View-process is running, connected and ready to respond.
    pub fn is_connected(&self) -> bool {
        matches!(self.view_state, ViewState::RunningAndConnected)
    }

    /// View-process generation.
    pub fn generation(&self) -> ViewProcessGen {
        self.generation
    }

    /// If is running in headless mode.
    pub fn headless(&self) -> bool {
        self.headless
    }

    /// If is running both view and app in the same process.
    pub fn same_process(&self) -> bool {
        self.same_process
    }

    fn disconnected_err(&self) -> Result<(), ipc::ViewChannelError> {
        if self.is_connected() {
            Ok(())
        } else {
            Err(ipc::ViewChannelError::Disconnected)
        }
    }

    fn try_talk(&mut self, req: Request) -> ipc::IpcResult<Response> {
        self.request_sender.send(req)?;
        self.response_receiver.recv()
    }
    pub(crate) fn talk(&mut self, req: Request) -> VpResult<Response> {
        debug_assert!(req.expect_response());

        if req.must_be_connected() {
            self.disconnected_err()?;
        }

        match self.try_talk(req) {
            Ok(r) => Ok(r),
            Err(ipc::ViewChannelError::Disconnected) => {
                self.handle_disconnect(self.generation);
                Err(ipc::ViewChannelError::Disconnected)
            }
        }
    }

    pub(crate) fn command(&mut self, req: Request) -> VpResult<()> {
        debug_assert!(!req.expect_response());

        if req.must_be_connected() {
            self.disconnected_err()?;
        }

        match self.request_sender.send(req) {
            Ok(_) => Ok(()),
            Err(ipc::ViewChannelError::Disconnected) => {
                self.handle_disconnect(self.generation);
                Err(ipc::ViewChannelError::Disconnected)
            }
        }
    }

    fn spawn_view_process(
        view_process_exe: &Path,
        view_process_env: &HashMap<Txt, Txt>,
        headless: bool,
    ) -> AnyResult<(
        Option<std::process::Child>,
        ipc::RequestSender,
        ipc::ResponseReceiver,
        ipc::EventReceiver,
    )> {
        let _span = tracing::trace_span!("spawn_view_process").entered();

        let init = ipc::AppInit::new();

        // create process and spawn it, unless is running in same process mode.
        let process = if ViewConfig::is_awaiting_same_process() {
            ViewConfig::set_same_process(ViewConfig {
                version: crate::VERSION.into(),
                server_name: Txt::from_str(init.name()),
                headless,
            });
            None
        } else {
            #[cfg(not(ipc))]
            {
                let _ = (view_process_exe, view_process_env);
                panic!("expected only same_process mode with `ipc` feature disabled");
            }

            #[cfg(ipc)]
            {
                let mut process = std::process::Command::new(view_process_exe);
                for (name, val) in view_process_env {
                    process.env(name, val);
                }
                let process = process
                    .env(VIEW_VERSION, crate::VERSION)
                    .env(VIEW_SERVER, init.name())
                    .env(VIEW_MODE, if headless { "headless" } else { "headed" })
                    .env("RUST_BACKTRACE", "full")
                    .spawn()?;
                Some(process)
            }
        };

        let (req, rsp, ev) = match init.connect() {
            Ok(r) => r,
            Err(e) => {
                #[cfg(ipc)]
                if let Some(mut p) = process {
                    if let Err(ke) = p.kill() {
                        tracing::error!(
                            "failed to kill new view-process after failing to connect to it\n connection error: {e:?}\n kill error: {ke:?}",
                        );
                    } else {
                        match p.wait() {
                            Ok(output) => {
                                let code = output.code();
                                if ViewConfig::is_version_err(code, None) {
                                    let code = code.unwrap_or(1);
                                    tracing::error!(
                                        "view-process API version mismatch, the view-process build must use the same exact version as the app-process, \
                                                will exit app-process with code 0x{code:x}"
                                    );
                                    zng_env::exit(code);
                                } else {
                                    tracing::error!("view-process exit code: {}", output.code().unwrap_or(1));
                                }
                            }
                            Err(e) => {
                                tracing::error!("failed to read output status of killed view-process, {e}");
                            }
                        }
                    }
                } else {
                    tracing::error!("failed to connect with same process");
                }
                return Err(e);
            }
        };

        Ok((process, req, rsp, ev))
    }

    /// Handle an [`Event::Inited`].
    ///
    /// Set the connected flag to `true`.
    pub fn handle_inited(&mut self, vp_gen: ViewProcessGen) {
        match self.view_state {
            ViewState::NotRunning => {
                if self.generation == vp_gen {
                    // crash respawn already sets gen
                    self.view_state = ViewState::RunningAndConnected;
                }
            }
            ViewState::Suspended => {
                self.generation = vp_gen;
                self.view_state = ViewState::RunningAndConnected;
            }
            ViewState::RunningAndConnected => {}
        }
    }

    /// Handle an [`Event::Suspended`].
    ///
    /// Set the connected flat to `false`.
    pub fn handle_suspended(&mut self) {
        self.view_state = ViewState::Suspended;
    }

    /// Handle an [`Event::Disconnected`].
    ///
    /// The `gen` parameter is the generation provided by the event. It is used to determinate if the disconnect has
    /// not been handled already.
    ///
    /// Tries to cleanup the old view-process and start a new one, if all is successful an [`Event::Inited`] is send.
    ///
    /// The old view-process exit code and std output is logged using the `vp_respawn` target.
    ///
    /// Exits the current process with code `1` if the view-process was killed by the user. In Windows this is if
    /// the view-process exit code is `1`. In Unix if it was killed by SIGKILL, SIGSTOP, SIGINT.
    ///
    /// # Panics
    ///
    /// If the last five respawns happened all within 500ms of the previous respawn.
    ///
    /// If the an error happens three times when trying to spawn the new view-process.
    ///
    /// If another disconnect happens during the view-process startup dialog.
    pub fn handle_disconnect(&mut self, vp_gen: ViewProcessGen) {
        if vp_gen == self.generation {
            #[cfg(not(ipc))]
            {
                tracing::error!(target: "vp_respawn", "cannot recover in same_process mode (no ipc)");
            }

            #[cfg(ipc)]
            {
                self.respawn_impl(true)
            }
        }
    }

    /// Reopen the view-process, causing another [`Event::Inited`].
    ///
    /// This is similar to [`handle_disconnect`] but the current process does not
    /// exit depending on the view-process exit code.
    ///
    /// [`handle_disconnect`]: Controller::handle_disconnect
    pub fn respawn(&mut self) {
        #[cfg(not(ipc))]
        {
            tracing::error!(target: "vp_respawn", "cannot recover in same_process mode (no ipc)");
        }

        #[cfg(ipc)]
        self.respawn_impl(false);
    }
    #[cfg(ipc)]
    fn respawn_impl(&mut self, is_crash: bool) {
        use zng_unit::TimeUnits;

        self.view_state = ViewState::NotRunning;
        self.is_respawn = true;

        let mut process = if let Some(p) = self.process.lock().take() {
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

        if is_crash {
            let t = Instant::now();
            if let Some(last_respawn) = self.last_respawn {
                if t - last_respawn < Duration::from_secs(60) {
                    self.fast_respawn_count += 1;
                    if self.fast_respawn_count == 2 {
                        panic!("disconnect respawn happened 2 times in less than 1 minute");
                    }
                } else {
                    self.fast_respawn_count = 0;
                }
            }
            self.last_respawn = Some(t);
        } else {
            self.last_respawn = None;
        }

        // try exit
        let mut killed_by_us = false;
        if !is_crash {
            let _ = process.kill();
            killed_by_us = true;
        } else if !matches!(process.try_wait(), Ok(Some(_))) {
            // if not exited, give the process 300ms to close with the preferred exit code.
            thread::sleep(300.ms());

            if !matches!(process.try_wait(), Ok(Some(_))) {
                // if still not exited, kill it.
                killed_by_us = true;
                let _ = process.kill();
            }
        }

        let code_and_output = match process.wait() {
            Ok(c) => Some(c),
            Err(e) => {
                tracing::error!(target: "vp_respawn", "view-process could not be killed, will abandon running, {e:?}");
                None
            }
        };

        // try print stdout/err and exit code.
        if let Some(c) = code_and_output {
            tracing::info!(target: "vp_respawn", "view-process killed");

            let code = c.code();
            #[allow(unused_mut)]
            let mut signal = None::<i32>;

            if !killed_by_us {
                // check if user killed the view-process, in this case we exit too.

                #[cfg(windows)]
                if code == Some(1) {
                    tracing::warn!(target: "vp_respawn", "view-process exit code (1), probably killed by the system, \
                                        will exit app-process with the same code");
                    zng_env::exit(1);
                }

                #[cfg(unix)]
                if code.is_none() {
                    use std::os::unix::process::ExitStatusExt as _;
                    signal = c.signal();

                    if let Some(sig) = signal
                        && [2, 9, 17, 19, 23].contains(&sig)
                    {
                        tracing::warn!(target: "vp_respawn", "view-process exited by signal ({sig}), \
                                            will exit app-process with code 1");
                        zng_env::exit(1);
                    }
                }
            }

            if !killed_by_us {
                let code = code.unwrap_or(0);
                let signal = signal.unwrap_or(0);
                tracing::error!(target: "vp_respawn", "view-process exit code: {code:#X}, signal: {signal}");
            }

            if ViewConfig::is_version_err(code, None) {
                let code = code.unwrap_or(1);
                tracing::error!(target: "vp_respawn", "view-process API version mismatch, the view-process build must use the same exact version as the app-process, \
                                        will exit app-process with code 0x{code:x}");
                zng_env::exit(code);
            }
        } else {
            tracing::error!(target: "vp_respawn", "failed to kill view-process, will abandon it running and spawn a new one");
        }

        // recover event listener closure (in a box).
        let on_event = match self.event_listener.take().unwrap().join() {
            Ok(fn_) => fn_,
            Err(p) => panic::resume_unwind(p),
        };

        // respawn
        let mut retries = 3;
        let (new_process, request, response, event_listener) = loop {
            match Self::spawn_view_process(&self.view_process_exe, &self.view_process_env, self.headless) {
                Ok(r) => break r,
                Err(e) => {
                    tracing::error!(target: "vp_respawn", "failed to respawn, {e:?}");
                    retries -= 1;
                    if retries == 0 {
                        panic!("failed to respawn `view-process` after 3 retries");
                    }
                    tracing::info!(target: "vp_respawn", "retrying respawn");
                }
            }
        };
        debug_assert!(new_process.is_some());

        // update connections
        self.process = Arc::new(Mutex::new(new_process));
        self.request_sender = request;
        self.response_receiver = response;

        let next_id = self.generation.next();
        self.generation = next_id;

        if let Err(ipc::ViewChannelError::Disconnected) = self.try_init() {
            panic!("respawn on respawn startup");
        }

        let ev = Self::spawn_other_process_listener(on_event, event_listener, self.process.clone());
        self.event_listener = Some(ev);
    }
}
impl Drop for Controller {
    /// Kills the View Process, unless it is running in the same process.
    fn drop(&mut self) {
        let _ = self.exit();
        #[cfg(ipc)]
        if let Some(mut process) = self.process.lock().take()
            && process.try_wait().is_err()
        {
            std::thread::sleep(Duration::from_secs(1));
            if process.try_wait().is_err() {
                tracing::error!("view-process did not exit after 1s, killing");
                let _ = process.kill();
                let _ = process.wait();
            }
        }
    }
}
