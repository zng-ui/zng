#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
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
    AppExtension,
    event::{event, event_args},
    handler::{async_app_hn, clmv},
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
#[derive(Default)]
pub struct SingleInstanceManager {}
impl AppExtension for SingleInstanceManager {
    fn init(&mut self) {
        let args: Box<[_]> = std::env::args().map(Txt::from).collect();
        APP_INSTANCE_EVENT.notify(AppInstanceArgs::now(args, 0usize));

        let name = match SINGLE_INSTANCE.lock().as_ref().map(|l| l.name.clone()) {
            Some(n) => n,
            None => return, // app is running in a special process, like a crash dialog
        };

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

        fn delivery_list(&self, _list: &mut UpdateDeliveryList) {}
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

zng_env::on_process_start!(|args| {
    if args.next_handlers_count > 0 && args.yield_count < zng_env::ProcessStartArgs::MAX_YIELD_COUNT {
        // absolute sure that this is the app-process
        return args.yield_once();
    }

    let mut lock = SINGLE_INSTANCE.lock();
    assert!(lock.is_none(), "single_instance already called in this process");

    let name = std::env::current_exe()
        .and_then(dunce::canonicalize)
        .expect("current exe is required")
        .display()
        .to_txt();
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
        tracing::info!("another instance running, will send args and exit");

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
                Ok(_) => zng_env::exit(0),
                Err(e) => {
                    eprintln!("error writing args (retries: {i}), {e}");
                }
            }
        }
        zng_env::exit(1);
    }
});

struct SingleInstanceData {
    _lock: single_instance::SingleInstance,
    name: Txt,
}

static SINGLE_INSTANCE: parking_lot::Mutex<Option<SingleInstanceData>> = parking_lot::Mutex::new(None);
