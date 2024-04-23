//! Demonstrates app-process crash handler and view-process respawn.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::{
    color::{filter::opacity, gradient::stops},
    layout::size,
    markdown::Markdown,
    prelude::*,
};
use zng_app::view_process::VIEW_PROCESS;
use zng_view::extensions::ViewExtensions;

fn main() {
    examples_util::print_info();

    // init crash_handler before view to use different view for the crash dialog app.
    zng::app::crash_handler(zng::app::CrashConfig::new(app_crash_dialog));

    // this is the normal app-process:

    // init view with extensions used to cause a crash in the view-process.
    zng_view::init_extended(test_extensions);

    APP.defaults().run_window(async {
        Window! {
            title = "Respawn Example";
            icon = WindowIcon::render(icon);
            start_position = window::StartPosition::CenterMonitor;
            widget::foreground = window_status();
            child_align = Align::TOP;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                layout::margin = 10;
                spacing = 5;
                children_align = Align::TOP;
                children = ui_vec![
                    Markdown! {
                        txt = "The renderer and OS windows are created in separate process, the `view-process`. \
                               It automatically respawns in case of a graphics driver crash or other similar fatal error.";
                    },
                    view_respawn(),
                    view_crash(),
                    Markdown! {
                        txt = "When the app is instantiated the crash handler takes over the process, becoming the `monitor-process`. \
                               It spawns the `app-process` that is the normal execution. If the `app-process` crashes it spawns the \
                               `dialog-process` that runs a different app that shows an error message.";
                    },
                    app_crash(),
                    Markdown! {
                        txt = "The states of these buttons is only preserved for `view-process` crashes. \
                               use `CONFIG` or some other state saving to better recover from `app-process` crashes.";
                    },
                    click_counter(),
                    click_counter(),
                    image(),
                ];
            };
        }
    });
}

// Crash dialog app, runs in the dialog-process.
fn app_crash_dialog(args: zng::app::CrashArgs) -> ! {
    zng::view_process::prebuilt::init();
    APP.defaults().run_window(async move {
        Window! {
            title = "Respawn Example - App Crashed";
            icon = WindowIcon::render(icon);
            start_position = zng::window::StartPosition::CenterMonitor;

            on_load = hn_once!(|_| {
                // force to foreground
                let _ = WINDOWS.focus(WINDOW.id());
            });
            on_close = hn_once!(args, |_| {
                args.exit(0);
            });

            padding = 5;
            child_top = Markdown!("### App Crashed\n\nThe Respawn Example app has crashed.\n\n#### Details:\n"), 5;
            child = Scroll! {
                padding = 5;
                child = zng::ansi_text::AnsiText!(args.latest().to_txt());
                widget::background_color = colors::BLACK;
            };
            child_bottom = Stack! {
                spacing = 5;
                direction = StackDirection::start_to_end();
                layout::align = Align::END;
                children = ui_vec![
                    Button! {
                        child = Text!("Crash Dialog");
                        on_click = hn_once!(|_| {
                            panic!("Test dialog-process crash!");
                        });
                    },
                    zng::rule_line::vr::Vr!(),
                    Button! {
                        child = Text!("Restart App");
                        on_click = hn_once!(args, |_| {
                            args.restart();
                        });
                    },
                    Button! {
                        child = Text!("Exit App");
                        on_click = hn_once!(args, |_| {
                            args.exit(0);
                        });
                    }
                ];
            }, 5;
        }
    });
    panic!("dialog app did not respond correctly")
}

fn view_respawn() -> impl UiNode {
    Button! {
        child = Text!("Respawn View-Process (F5)");
        gesture::click_shortcut = shortcut!(F5);
        on_click = hn!(|_| {
            VIEW_PROCESS.respawn();
        });
    }
}

fn view_crash() -> impl UiNode {
    Button! {
        child = Text!("Crash View-Process");
        on_click = hn!(|_| {
            if let Ok(Some(ext)) = VIEW_PROCESS.extension_id("zng.examples.respawn.crash") {
                let _ = VIEW_PROCESS.app_extension::<_, ()>(ext, &());
            } else {
                tracing::error!(r#"extension "zng-view.crash" unavailable"#)
            }
        });
    }
}

fn app_crash() -> impl UiNode {
    Button! {
        child = Text!("Crash App-Process");
        on_click = hn!(|_| {
            panic!("Test app-process crash!");
        });
    }
}

fn click_counter() -> impl UiNode {
    let t = var_from("Click Me!");
    let mut count = 0;

    Button! {
        on_click = hn!(t, |_| {
            count += 1;
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 {"s"} else {""});
            t.set(new_txt);
        });
        child = Text!(t);
    }
}

fn image() -> impl UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 3;
        children = ui_vec![
            text::Strong!("Image:"),
            Image! { source = include_bytes!("res/window/icon-bytes.png"); size = (32, 32); },
        ];
    }
}

fn window_status() -> impl UiNode {
    let vars = WINDOW.vars();

    macro_rules! status {
        ($name:ident) => {
            Text!(vars.$name().map(|v| formatx!("{}: {v:?}", stringify!($name))))
        };
    }

    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        layout::margin = 10;
        layout::align = Align::BOTTOM_START;
        widget::background_color = color::color_scheme_map(colors::WHITE.with_alpha(10.pct()), colors::BLACK.with_alpha(10.pct()));
        text::font_family = "monospace";
        opacity = 80.pct();
        children = ui_vec![
            status!(actual_position),
            status!(actual_size),
            status!(restore_state),
            status!(restore_rect),
        ]
    }
}

fn icon() -> impl UiNode {
    Container! {
        size = (36, 36);
        widget::background_gradient = layout::Line::to_bottom_right(), stops![web_colors::ORANGE_RED, 70.pct(), web_colors::DARK_RED];
        widget::corner_radius = 6;
        text::font_size = 28;
        text::font_weight = FontWeight::BOLD;
        child_align = Align::CENTER;
        child = Text!("R");
    }
}

fn test_extensions() -> ViewExtensions {
    let mut ext = ViewExtensions::new();
    ext.command::<(), ()>("zng.examples.respawn.crash", |_, _| panic!("Test view-process crash!"));
    ext
}
