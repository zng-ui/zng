// see ../../render.rs

use std::env;

use color_print::cstr;
use zng::{
    app::HeadlessApp,
    layout::{FactorUnits as _, TimeUnits as _},
    view_process,
    window::RenderMode,
    APP,
};

mod tests;

fn main() {
    if let Ok(vp) = env::var("RENDER_TESTS_VP") {
        return run(vp.into());
    }

    for view_process in ViewProcess::OPTIONS {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .env("RENDER_TESTS_VP", format!("{view_process:?}"))
            // .env("RUST_BACKTRACE", "1")
            .status()
            .unwrap();
        assert!(result.success());
    }
}

fn run(view_process: ViewProcess) {
    match view_process {
        ViewProcess::DefaultInit => {
            view_process::default::init();
            run_tests(view_process, APP.defaults().run_headless(true));
        }
        ViewProcess::DefaultSame => {
            view_process::default::run_same_process(move || run_tests(view_process, APP.defaults().run_headless(true)))
        }
        ViewProcess::PrebuiltInit => {
            view_process::prebuilt::init();
            run_tests(view_process, APP.defaults().run_headless(true));
        }
        ViewProcess::PrebuiltSame => {
            view_process::prebuilt::run_same_process(move || run_tests(view_process, APP.defaults().run_headless(true)))
        }
    }
}

fn run_tests(view_process: ViewProcess, mut app: HeadlessApp) {
    let test = [("bw_rgb", tests::bw_rgb)];
    let render_mode = [RenderMode::Software, RenderMode::Dedicated, RenderMode::Integrated];
    let scale_factor = [1.fct(), 1.5.fct(), 2.fct()];

    for (test_name, test) in test {
        for render_mode in render_mode {
            for scale_factor in scale_factor {
                println!(
                    cstr!("\n<bold><green>TEST</> {}({:?}, {:?}, {:?})</>"),
                    test_name, view_process, render_mode, scale_factor
                );

                let task = zng::task::run_catch(async move { test(render_mode, scale_factor).await });
                let result = app.run_task(task).unwrap();

                if result.is_err() {
                    println!(cstr!("<bold><red>FAILED</></>"));
                }

                zng::image::IMAGES.clean_all();
            }
        }
    }

    app.exit();

    std::thread::spawn(move || {
        std::thread::sleep(10.secs());
        eprintln!(
            cstr!("<bold><red>TIMEOUT</>: </> view-process {:?} did not exit after app exit",),
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
