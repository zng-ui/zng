//! Demonstrates app-process crash handler and view-process respawn.

use zng::{
    color::{filter::opacity, gradient::stops},
    layout::size,
    markdown::Markdown,
    prelude::*,
};
use zng_app::view_process::VIEW_PROCESS;

fn main() {
    // log other processes too.
    zng::app::print_tracing(tracing::Level::INFO);
    // init metadata, view-process, crash-dialog-process.
    zng::env::init!();

    // this is the normal app-process:
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
                spacing = 10;
                children_align = Align::TOP;
                children = ui_vec![
                    Markdown! {
                        layout::margin = (20, 0, 0, 0);
                        txt = "The renderer and OS windows are created in separate process, the `view-process`. \
                               It automatically respawns in case of a graphics driver crash or other similar fatal error.";
                    },
                    Wrap! {
                        spacing = 5;
                        children = ui_vec![view_respawn(), view_crash(),];
                    },
                    Markdown! {
                        layout::margin = (20, 0, 0, 0);
                        txt = "When the app is instantiated the crash handler takes over the process, becoming the `monitor-process`. \
                               It spawns the `app-process` that is the normal execution. If the `app-process` crashes it spawns the \
                               `dialog-process` that runs a different app that shows an error message.";
                    },
                    Wrap! {
                        spacing = 5;
                        children = ui_vec![
                            app_crash("panic"),
                            app_crash("access violation"),
                            app_crash("stack overflow"),
                            app_crash("custom exit"),
                        ];
                    },
                    Markdown! {
                        layout::margin = (20, 0, 0, 0);
                        txt = "The states of these buttons is only preserved for `view-process` crashes. \
                               use `CONFIG` or some other state saving to better recover from `app-process` crashes.";
                    },
                    Wrap! {
                        spacing = 5;
                        children = ui_vec![click_counter(), image(), click_counter(),];
                    },
                ];
            };
            // widget::on_init = {
            //     let respawn_on_init = true;
            //     hn!(|_| {
            //         if std::mem::take(&mut respawn_on_init) {
            //             VIEW_PROCESS.respawn();
            //         }
            //     })
            // };
        }
    });
}

// Crash dialog app, runs in the dialog-process.
// zng::app::crash_handler::crash_handler_config!(|cfg| cfg.dialog(app_crash_handler));
#[allow(unused)]
fn app_crash_dialog(args: zng::app::crash_handler::CrashArgs) {
    APP.defaults().run_window(async move {
        Window! {
            title = "Respawn Example - App Crashed";
            icon = WindowIcon::render(icon);

            start_position = zng::window::StartPosition::CenterMonitor;
            auto_size = window::AutoSize::CONTENT;
            min_size = (300, 100);
            enabled_buttons = !window::WindowButton::MAXIMIZE;

            on_load = hn_once!(|_| {
                // force to foreground
                let _ = WINDOWS.focus(WINDOW.id());
            });

            padding = 5;
            child = Markdown!("The Respawn Example app has crashed.\n\n{}\n\n", args.latest().message());
            child_spacing = 5;
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
                    Vr!(),
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
            };
        }
    });
}

fn view_respawn() -> UiNode {
    Button! {
        child = Text!("Respawn View-Process (F5)");
        gesture::click_shortcut = shortcut!(F5);
        on_click = hn!(|_| {
            VIEW_PROCESS.respawn();
        });
    }
}

fn view_crash() -> UiNode {
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

fn app_crash(crash_name: &'static str) -> UiNode {
    Button! {
        child = Text!("Crash ({crash_name})");
        on_click = hn!(|_| {
            match crash_name {
                "panic" => panic!("Test app-process crash!"),
                "access violation" => {
                    // SAFETY: deliberate access violation
                    #[expect(deref_nullptr)]
                    unsafe {
                        *std::ptr::null_mut() = true;
                    }
                }
                "stack overflow" => {
                    fn overflow(c: u64) {
                        if c < u64::MAX {
                            overflow(c + 1)
                        }
                    }
                    overflow(0)
                }
                "custom exit" => {
                    eprintln!("custom error");
                    zng::env::exit(0xBAD);
                }
                n => panic!("Unknown crash '{n}'"),
            }
        });
    }
}

fn click_counter() -> UiNode {
    let t = var_from("Click Me!");
    let mut count = 0;

    Button! {
        on_click = hn!(t, |_| {
            count += 1;
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 { "s" } else { "" });
            t.set(new_txt);
        });
        child = Text!(t);
    }
}

fn image() -> UiNode {
    Image! {
        source = include_bytes!("../../window/res/icon-bytes.png");
        size = (32, 32);
        tooltip = Tip!(Text!("Image reloads after respawn"));
    }
}

fn window_status() -> UiNode {
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
        widget::background_color = light_dark(colors::BLACK.with_alpha(10.pct()), colors::WHITE.with_alpha(10.pct()));
        text::font_family = "monospace";
        opacity = 80.pct();
        children = ui_vec![
            status!(actual_position),
            status!(actual_size),
            status!(restore_state),
            status!(restore_rect),
        ];
    }
}

fn icon() -> UiNode {
    Container! {
        size = (36, 36);
        widget::background_gradient =
            layout::Line::to_bottom_right(),
            stops![web_colors::ORANGE_RED, 70.pct(), web_colors::DARK_RED],
        ;
        widget::corner_radius = 6;
        text::font_size = 28;
        text::font_weight = FontWeight::BOLD;
        child_align = Align::CENTER;
        child = Text!("R");
    }
}

// init view with extensions used to cause a crash in the view-process.
zng_view::view_process_extension!(|ext| {
    ext.command::<(), ()>("zng.examples.respawn.crash", |_, _| panic!("Test view-process crash!"));
});
