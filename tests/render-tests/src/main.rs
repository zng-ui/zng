// see ../../render.rs

use std::{
    env,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
    time::Instant,
};

use color_print::cstr;
use zng::{
    APP,
    app::HeadlessApp,
    layout::{FactorUnits as _, TimeUnits as _},
    text::{Txt, formatx},
    window::RenderMode,
};

mod tests;

static FAILED: AtomicBool = AtomicBool::new(false);

fn main() {
    zng::app::print_tracing(tracing::Level::INFO);

    std::thread::spawn(move || {
        std::thread::sleep(10.minutes());
        eprintln!(cstr!("<bold><red>TIMEOUT</>:</> render-tests did not exit after 10 minutes"),);
        std::process::exit(101);
    });

    let args = Args::parse();
    if let Ok(vp) = env::var("RENDER_TESTS_VP") {
        zng::env::init!();
        return run(args, vp.into());
    }

    for view_process in ViewProcess::OPTIONS {
        if args.no_prebuilt && view_process.is_prebuilt() {
            continue;
        }

        let view_process = format!("{view_process:?}");

        if args.include_vp(&view_process) {
            let mut retries = 0;
            while retries < 5 {
                // CI fails some times (view timeout even after 60s?)
                let result = std::process::Command::new(std::env::current_exe().unwrap())
                    .env("ZNG_VIEW_NO_INIT_START", "")
                    .env("ZNG_NO_CRASH_HANDLER", "")
                    .env("ZNG_VIEW_TIMEOUT", "60")
                    .env("RENDER_TESTS_VP", &view_process)
                    .args(std::env::args().skip(1))
                    // .env("RUST_BACKTRACE", "1")
                    .status()
                    .unwrap();

                if result.success() {
                    retries = 0;
                    break;
                } else {
                    retries += 1;
                    if retries < 4 {
                        eprintln!("\n\nfailed, retrying ({retries})..");
                        std::thread::sleep(std::time::Duration::new(retries + 1, 0));
                    }
                }
            }
            if retries > 0 {
                FAILED.store(true, Relaxed);
            }
        }
    }

    if FAILED.load(Relaxed) {
        std::process::exit(1)
    }
}

fn run(args: Args, view_process: ViewProcess) {
    match view_process {
        ViewProcess::DefaultInit => {
            zng_view::view_process_main();
            run_tests(args, view_process, APP.defaults().run_headless(true));
        }
        ViewProcess::DefaultSame => zng_view::run_same_process(move || run_tests(args, view_process, APP.defaults().run_headless(true))),
        ViewProcess::PrebuiltInit => {
            zng_view_prebuilt::view_process_main();
            run_tests(args, view_process, APP.defaults().run_headless(true));
        }
        ViewProcess::PrebuiltSame => {
            zng_view_prebuilt::run_same_process(move || run_tests(args, view_process, APP.defaults().run_headless(true)))
        }
    }
    if FAILED.load(Relaxed) {
        std::process::exit(1)
    }
}

fn run_tests(args: Args, view_process: ViewProcess, mut app: HeadlessApp) {
    SAVE.set(args.save);

    let test = [("bw_rgb", tests::bw_rgb)];
    let render_mode = [RenderMode::Software, RenderMode::Dedicated, RenderMode::Integrated];
    let scale_factor = [1.fct(), 1.5.fct(), 2.fct()];

    for (test_name, test) in test {
        for render_mode in render_mode {
            for scale_factor in scale_factor {
                let test_name = formatx!("{test_name}({view_process:?}, {render_mode:?}, {scale_factor:?})");
                if !args.include_test(&test_name) {
                    continue;
                }

                println!(cstr!("\n<bold><green>TEST</> {}</>"), test_name);
                TEST_NAME.set(test_name);

                let start = Instant::now();

                let task = zng::task::run_catch(async move { test(render_mode, scale_factor).await });
                let task = zng::task::with_deadline(task, 40.secs());
                let result = app.run_task(task).unwrap();

                match result {
                    Ok(result) => {
                        if result.is_err() {
                            println!(cstr!("<bold><red>FAILED</></>"));
                            FAILED.store(true, Relaxed);
                        } else {
                            println!(cstr!("<bold><green>PASSED</></> in {:?}"), start.elapsed());
                        }
                    }
                    Err(_) => {
                        eprintln!(cstr!("<bold><red>TIMEOUT</>:</> test did not complete in 40s"));
                        FAILED.store(true, Relaxed);
                    }
                }
                zng::image::IMAGES.clean_all();
            }
        }
    }

    app.exit();

    std::thread::spawn(move || {
        std::thread::sleep(10.secs());
        eprintln!(
            cstr!("<bold><red>TIMEOUT</>:</> view-process {:?} did not exit after app exit"),
            view_process
        );
        std::process::exit(101);
    });
}

#[derive(Debug, Clone, Copy)]
enum ViewProcess {
    DefaultInit,
    DefaultSame,
    PrebuiltInit,
    PrebuiltSame,
}
impl ViewProcess {
    pub fn is_prebuilt(self) -> bool {
        matches!(self, Self::PrebuiltInit | Self::PrebuiltSame)
    }
}
impl ViewProcess {
    const OPTIONS: [ViewProcess; 4] = [
        ViewProcess::DefaultInit,
        ViewProcess::DefaultSame,
        ViewProcess::PrebuiltInit,
        ViewProcess::PrebuiltSame,
    ];
}
impl From<String> for ViewProcess {
    fn from(value: String) -> Self {
        for o in Self::OPTIONS {
            if format!("{o:?}") == value {
                return o;
            }
        }
        panic!("{value}")
    }
}

pub fn save_name() -> Option<String> {
    if SAVE.get() {
        Some(format!("screenshot.{}", test_name_clean()))
    } else {
        None
    }
}

pub fn test_name_clean() -> String {
    TEST_NAME
        .get()
        .chars()
        .filter_map(|c| {
            if c == ' ' {
                None
            } else if c.is_alphanumeric() {
                Some(c)
            } else {
                Some('_')
            }
        })
        .collect()
}

zng::app::app_local! {
    pub static TEST_NAME: Txt = const { Txt::from_static("") };
    pub static SAVE: bool = const { false };
}

#[derive(Clone, Debug)]
struct Args {
    save: bool,
    no_prebuilt: bool,
    filter: Txt,
}
impl Args {
    fn parse() -> Self {
        let mut args = Args {
            save: false,
            no_prebuilt: false,
            filter: Txt::from_static(""),
        };

        for arg in std::env::args().skip(1) {
            match arg.as_str() {
                "--save" => args.save = true,
                "--no-prebuilt" => args.no_prebuilt = true,
                a => {
                    if a.starts_with('-') {
                        panic!("unexpected arg '{a}'");
                    }
                    args.filter = Txt::from_str(a);
                }
            }
        }

        args
    }

    fn include_vp(&self, view_process: &str) -> bool {
        self.filter.is_empty()
            || self.filter.contains(view_process)
            || ViewProcess::OPTIONS.iter().all(|o| !self.filter.contains(&format!("{o:?}")))
    }

    fn include_test(&self, name: &str) -> bool {
        self.filter.is_empty() || name.contains(self.filter.as_str())
    }
}
