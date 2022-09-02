use zero_ui::core::{
    context::WindowContext,
    keyboard::Keyboard,
    window::{Monitors, Window},
};
use zero_ui::prelude::*;

const SAME_PROCESS: bool = true;
const RENDER_MODE: RenderMode = RenderMode::Dedicated;

static TESTS: &[(&str, TestFn, FilterFn)] = &[
    ("text_change_all", text_change_all, default_filter),
    ("text_change_one", text_change_one, default_filter),
    ("multi_window", multi_window, default_filter),
];

/// Exclude trace and debug events.
///
/// Include trace spans above only above 1ms.
#[allow(unused)]
fn default_filter(a: profile_util::FilterArgs) -> bool {
    if a.is_span {
        a.duration > 1000
    } else {
        a.level < profile_util::Level::DEBUG
    }
}

fn text_change_all(ctx: &mut WindowContext) -> Window {
    let mut dots_count = 3;
    let msg = ctx.timers.interval(1.secs() / 60, false).map(move |_| {
        dots_count += 1;
        if dots_count == 8 {
            dots_count = 0;
        }
        formatx!("loading{:.^dots_count$}", "")
    });

    let mut texts = widget_vec![];
    for _ in 0..2000 {
        texts.push(text! {
            text = msg.clone();
            width = 80;
            when self.is_hovered {
                color = colors::RED;
            }
        });
    }

    window! {
        title = "stress - text_change_all";
        state = WindowState::Maximized;
        content = uniform_grid! {
            columns = 30;
            items = texts;
        };
    }
}

fn text_change_one(_ctx: &mut WindowContext) -> Window {
    let mut texts = widget_vec![];

    for _ in 0..2000 {
        texts.push(text! {
            text = "RED";
            width = 80;
            font_size = 16;
            font_weight = FontWeight::BOLD;
            when self.is_hovered {
                text = "HOT";
                color = colors::RED;
            }
        });
    }

    window! {
        title = "stress - text_change_one";
        state = WindowState::Maximized;
        content = uniform_grid! {
            columns = 30;
            items = texts;
        };
    }
}

fn multi_window(ctx: &mut WindowContext) -> Window {
    let mut dots_count = 3;
    let msg = ctx.timers.interval(1.secs() / 60, false).map(move |_| {
        dots_count += 1;
        if dots_count == 8 {
            dots_count = 0;
        }
        formatx!("loading{:.^dots_count$}", "")
    });

    let monitor_size = MonitorQuery::Primary
        .select(ctx.vars, Monitors::req(ctx.services))
        .unwrap()
        .size()
        .copy(ctx.vars);
    let window_size = PxSize::new(monitor_size.width / Px(5), monitor_size.height / Px(2));

    let mut window_pos = PxPoint::zero();

    let mut wns = vec![];
    for i in 0..10 {
        let mut texts = widget_vec![];
        for _ in 0..200 {
            texts.push(text! {
                text = msg.clone();
                width = 80;
                when self.is_hovered {
                    color = colors::RED;
                }
            });
        }

        wns.push(window! {
            title = formatx!("stress - multi_window - {i}");
            position = window_pos;
            size = window_size;
            on_close = hn!(|ctx, _| {
                if Keyboard::req(ctx.services).modifiers().get(ctx).is_empty() {
                    Windows::req(ctx.services).close_all();
                }
            });
            content = uniform_grid! {
                columns = 6;
                items = texts;
            };
        });

        window_pos.x += window_size.width;
        if window_pos.x + window_size.width > monitor_size.width {
            window_pos.x = Px(0);
            window_pos.y += window_size.height;
        }
    }

    let r = wns.pop().unwrap();

    let windows = Windows::req(ctx.services);
    for w in wns {
        windows.open(|_| w);
    }

    r
}

fn main() {
    if !SAME_PROCESS {
        #[cfg(feature = "ipc")]
        zero_ui_view::init();

        #[cfg(not(feature = "ipc"))]
        {
            panic!("only `SAME_PROCESS` supported with feature `ipc` disabled");
        }
    }

    let name;
    let test;
    let filter;

    if let Some(s) = std::env::args().nth(1) {
        if let Some((n, t, f)) = TESTS.iter().find(|(n, _, _)| *n == s.as_str()) {
            name = n;
            test = t;
            filter = f;
        } else {
            eprintln!("unknown stress test `{s}`\nTESTS:");
            for (t, _, _) in TESTS {
                eprintln!("   {t}");
            }
            return;
        }
    } else {
        eprintln!("do profile --stress <stress-test>\nTESTS:");
        for (t, _, _) in TESTS {
            eprintln!("   {t}");
        }
        return;
    }

    let profile_file = format!(
        "profile-stress-{}{}{}{}{}{}{}{}",
        name,
        if cfg!(debug_assertions) { "-dbg" } else { "" },
        if SAME_PROCESS { "" } else { "-no_vp" },
        if cfg!(feature = "ipc") { "-ipc" } else { "" },
        if cfg!(feature = "dyn_widget") { "-dynw" } else { "" },
        if cfg!(feature = "dyn_node") { "-dynp" } else { "" },
        if cfg!(feature = "dyn_app_extension") { "-dyna" } else { "" },
        match RENDER_MODE {
            RenderMode::Dedicated => "",
            RenderMode::Integrated => "-integrated",
            RenderMode::Software => "-software",
        }
    );
    let rec = profile_util::record_profile(
        &profile_file,
        &[
            ("stress-test", name),
            ("SAME_PROCESS", &SAME_PROCESS),
            ("ipc", &cfg!(feature = "ipc")),
            ("dyn_app_extension", &cfg!(feature = "dyn_app_extension")),
            ("dyn_widget", &cfg!(feature = "dyn_widget")),
            ("dyn_node", &cfg!(feature = "dyn_node")),
            ("render_mode", &format!("{RENDER_MODE:?}")),
        ],
        filter,
    );

    let run_app = move || {
        App::default().run_window(|ctx| {
            Windows::req(ctx.services).default_render_mode = RENDER_MODE;
            test(ctx)
        });
    };
    if SAME_PROCESS {
        zero_ui_view::run_same_process(run_app);
    } else {
        run_app();
    }

    println!("saving `{profile_file}`..");
    rec.finish();
}

type TestFn = fn(&mut WindowContext) -> Window;
type FilterFn = fn(profile_util::FilterArgs) -> bool;
