use zero_ui::core::{app::ShutdownRequestedEvent, context::WindowContext, window::Window};
use zero_ui::prelude::*;

const PROFILE: bool = true;

static TESTS: &[(&str, TestFn)] = &[("text_eq", text_eq)];

fn text_eq(ctx: &mut WindowContext) -> Window {
    let mut dots_count = 3;
    let msg = ctx.timers.interval(300.ms(), true).map(move |_| {
        dots_count += 1;
        if dots_count == 8 {
            dots_count = 0;
        }
        formatx!("loading{:.^1$}", "", dots_count)
    });

    let mut texts = widget_vec![];

    for _ in 0..3000 {
        texts.push(text! {
            text = msg.clone();
            width = 80;
        });
    }

    window! {
        title = "stress - text_eq";
        state = WindowState::Maximized;
        content = uniform_grid! {
            columns = 30;
            items = texts;
        };
    }
}

fn main() {
    let name;
    let test;

    if let Some(s) = std::env::args().nth(1) {
        if let Some((n, t)) = TESTS.iter().find(|(n, _)| *n == s.as_str()) {
            name = n;
            test = t;
        } else {
            eprintln!("unknown stress test `{}`\nTESTS:", s);
            for (t, _) in TESTS {
                eprintln!("   {}", t);
            }
            return;
        }
    } else {
        eprintln!("do run stress -- <stress-test>\nTESTS:");
        for (t, _) in TESTS {
            eprintln!("   {}", t);
        }
        return;
    }

    if PROFILE {
        let rec = examples_util::record_profile(format!("profile-stress-{}.json.gz", name), &[("stress-test", name)]);
        zero_ui_view::run_same_process(move || {
            App::default().run_window(move |ctx| {
                ctx.events
                    .on_event(
                        ShutdownRequestedEvent,
                        app_hn_once!(|_, _| {
                            rec.finish();
                        }),
                    )
                    .permanent();

                test(ctx)
            });
        });
    } else {
        examples_util::print_info();
        zero_ui_view::run_same_process(move || {
            App::default().run_window(test);
        });
    }
}

type TestFn = fn(&mut WindowContext) -> Window;
