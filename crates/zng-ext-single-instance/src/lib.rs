#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
//!
//! Single app-process instance mode.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]

use std::{
    io::{Read, Write},
    time::Duration,
};

use zng_app::{
    event::{event, event_args},
    handler::{async_app_hn, clmv},
    AppExtension,
};
use zng_ext_fs_watcher::WATCHER;
use zng_txt::{ToTxt, Txt};

/// Single instance event manager.
///
/// # Events
///
/// Events this extension provides.
///
/// * [`APP_INSTANCE_EVENT`]
///
/// Note that the event will notify even if [`single_instance()`] is not called, but it must be
/// called to make the app actually single instance.
#[derive(Default)]
pub struct SingleInstanceManager {}
impl AppExtension for SingleInstanceManager {
    fn init(&mut self) {
        let args: Box<[_]> = std::env::args().map(Txt::from).collect();
        APP_INSTANCE_EVENT.notify(AppInstanceArgs::now(args, 0usize));

        if let Some(name) = SINGLE_INSTANCE.lock().as_ref().map(|l| l.name.clone()) {
            let args_file = std::env::temp_dir().join(name);
            let mut count = 1usize;
            WATCHER
                .on_file_changed(
                    &args_file,
                    async_app_hn!(args_file, |_, _| {
                        let args = zng_task::wait(clmv!(args_file, || {
                            for i in 0..5 {
                                if i > 0 {
                                    std::thread::sleep(Duration::from_millis(200));
                                }

                                // take args
                                // read all text and truncates the file
                                match std::fs::File::options().read(true).write(true).open(&args_file) {
                                    Ok(mut file) => {
                                        let mut s = String::new();
                                        if let Err(e) = file.read_to_string(&mut s) {
                                            tracing::error!("error reading args (retry {i}), {e}");
                                            continue;
                                        }
                                        file.set_len(0).unwrap();
                                        return s;
                                    }
                                    Err(e) => {
                                        if e.kind() == std::io::ErrorKind::NotFound {
                                            return String::new();
                                        }
                                        tracing::error!("error reading args (retry {i}), {e}")
                                    }
                                }
                            }
                            String::new()
                        }))
                        .await;

                        // parse args
                        for line in args.lines() {
                            let line = line.trim();
                            if line.is_empty() {
                                continue;
                            }

                            let args = match serde_json::from_str::<Box<[Txt]>>(line) {
                                Ok(args) => args,
                                Err(e) => {
                                    tracing::error!("invalid args, {e}");
                                    Box::new([])
                                }
                            };

                            APP_INSTANCE_EVENT.notify(AppInstanceArgs::now(args, count));

                            count += 1;
                        }
                    }),
                )
                .perm();
        } else {
            tracing::warn!("using `SingleInstanceManager` without calling `single_instance()`");
        }
    }
}

event_args! {
    /// Arguments for [`APP_INSTANCE_EVENT`].
    pub struct AppInstanceArgs {
        /// Arguments the app instance was started with.
        ///
        /// See [`std::env::args`] for more details.
        pub args: Box<[Txt]>,

        /// Instance count. Is zero for the current process, in single instance mode
        /// increments for each subsequent attempt to instantiate the app.
        pub count: usize,

        ..

        fn delivery_list(&self, _list: &mut UpdateDeliveryList) { }
    }
}
impl AppInstanceArgs {
    /// If the arguments are for the currently executing process (main).
    ///
    /// This is only `true` once, on the first event on startup.
    pub fn is_current(&self) -> bool {
        self.count == 0
    }
}

event! {
    /// App instance init event, with the arguments.
    ///
    /// This event notifies once on start. If the app is "single instance" this event will also notify for each
    /// new attempt to instantiate while the current process is already running.
    pub static APP_INSTANCE_EVENT: AppInstanceArgs;
}

/// Enable single instance mode for the app-process.
///
/// The current executable path is used as the unique name. See [`single_instance_named`] for more details about name.
///
/// # Examples
///
/// The example below demonstrates a single instance process setup. The [`single_instance()`] function is called before
/// the app starts building. After the app starts building, before run, a subscription for [`APP_INSTANCE_EVENT`] is setup,
/// this event will receive args for the current instance on run and for other instances latter.
///
/// ```
/// # mod zng { pub(crate) use zng_ext_single_instance as app; }
/// # mod view_process { pub fn init() { } }
/// # trait FakeDefaults { fn defaults(self) -> zng_app::AppExtended<impl zng_app::AppExtension>; }
/// # impl FakeDefaults for zng_app::APP { fn defaults(self) -> zng_app::AppExtended<impl zng_app::AppExtension> { self.minimal() } }
/// # use zng_app::{APP, handler::app_hn};
/// fn main() {
///     view_process::init();
///     // zng::task::ipc::run_worker(worker);
///
///     // must be called after `view_process`, `run_worker` and before the APP build.
///     zng::app::single_instance();
///
///     app_main();
/// }
///
/// fn app_main() {
///     let app = APP.defaults();
///
///     zng::app::APP_INSTANCE_EVENT
///         .on_event(app_hn!(|args: &zng::app::AppInstanceArgs, _| {
///             println!("app instance #{}, args: {:?}", args.count, args.args);
///         }))
///         .perm();
///
/// # macro_rules! demo { () => {
///     app.run_window(async {
///         Window! {
///             child_align = Align::CENTER;
///             child = Button! {
///                 child = Text!("Spawn Instance");
///                 on_click = hn!(|_| {
///                     let exe = std::env::current_exe().unwrap();
///                     std::process::Command::new(exe).arg("app arg 1").arg("--arg2").spawn().unwrap();
///                 });
///             }
///         }
///     });
/// # }}
/// }
/// ```
pub fn single_instance() {
    single_instance_named(
        std::env::current_exe()
            .and_then(|p| p.canonicalize())
            .expect("current exe is required")
            .display()
            .to_txt(),
    )
}

/// Enable single instance mode for the app-process, using a custom unique identifier for the app.
///
/// The name should have only ASCII alphanumerics and be at maximum 128 bytes in length only, otherwise it will be mangled.
/// The name is used to create a global lock, see the [single-instance] crate for more details.
///
/// [single-instance]: https://docs.rs/single-instance/
pub fn single_instance_named(unique_name: impl Into<Txt>) {
    single_instance_impl(unique_name.into())
}

/// If single instance mode is enabled and the current process is the instance.
///
/// This is only valid after calling [`single_instance`] or [`single_instance_named`].
///
/// [`single_instance`]: fn@single_instance
pub fn is_single_instance() -> bool {
    SINGLE_INSTANCE.lock().is_some()
}

fn single_instance_impl(name: Txt) {
    let mut lock = SINGLE_INSTANCE.lock();
    assert!(lock.is_none(), "single_instance already called in this process");

    let name: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .collect();
    let mut name = name.as_str();
    if name.len() > 128 {
        name = &name[name.len() - 128..];
    }
    let name = zng_txt::formatx!("zng-si-{name}");

    let l = single_instance::SingleInstance::new(&name).expect("failed to create single instance lock");

    if l.is_single() {
        *lock = Some(SingleInstanceData { _lock: l, name });
    } else {
        tracing::info!("another instance running, will send args an exit");

        let args: Box<[_]> = std::env::args().collect();
        let args = format!("\n{}\n", serde_json::to_string(&args).unwrap());

        let try_write = move || -> std::io::Result<()> {
            let mut file = std::fs::File::options()
                .create(true)
                .append(true)
                .open(std::env::temp_dir().join(name.as_str()))?;
            file.write_all(args.as_bytes())
        };

        for i in 0..5 {
            if i > 0 {
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
            match try_write() {
                Ok(_) => std::process::exit(0),
                Err(e) => {
                    eprintln!("error writing args (retries: {i}), {e}");
                }
            }
        }
        std::process::exit(1);
    }
}

struct SingleInstanceData {
    _lock: single_instance::SingleInstance,
    name: Txt,
}

static SINGLE_INSTANCE: parking_lot::Mutex<Option<SingleInstanceData>> = parking_lot::Mutex::new(None);
