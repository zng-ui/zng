#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::app::view_process::ViewProcessExt;
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();
    App::default().run_window(|ctx| {
        window! {
            title = "View-Process Respawn Example";
            start_position = StartPosition::CenterMonitor;
            on_key_down = hn!(|ctx, args: &KeyInputArgs| {
                if args.key == Some(Key::F5) {
                    ctx.services.view_process().respawn();
                }
            });
            foreground = window_status(ctx);
            content = v_stack! {
                spacing = 5;
                items_align = Alignment::TOP;
                items = widgets![
                    text(
                        "The renderer and OS windows are created in another process, the `view-process`,\n\
                        it automatically respawns in case of a graphics driver crash or other similar fatal error.\n"
                    ),
                    respawn(),
                    #[cfg(debug_assertions)]
                    crash_respawn(),
                    click_counter(),
                    click_counter(),
                    image(),
                ];
            };
        }
    });
}

fn respawn() -> impl Widget {
    button! {
        content = text("Respawn (F5)");
        on_click = hn!(|ctx, _| {
            ctx.services.view_process().respawn();
        });
    }
}

#[cfg(debug_assertions)]
fn crash_respawn() -> impl Widget {
    button! {
        content = text("Crash View-Process");
        on_click = hn!(|ctx, _| {
            ctx.services.view_process().crash_view_process();
        });
    }
}

fn click_counter() -> impl Widget {
    let t = var_from("Click Me!");
    let mut count = 0;

    button! {
        on_click = hn!(t, |ctx, _| {
            count += 1;
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 {"s"} else {""});
            t.set(ctx, new_txt);
        });
        content = text(t);
    }
}

fn image() -> impl Widget {
    v_stack! {
        spacing = 3;
        items = widgets![
            strong("Image:"),
            image! { source = "examples/res/window/icon-bytes.png"; size = (32, 32); },
        ];
    }
}

fn window_status(ctx: &mut WindowContext) -> impl Widget {
    let vars = ctx.window_state.req(WindowVarsKey);

    macro_rules! status {
        ($name:ident) => {
            text(vars.$name().map(|v| formatx!("{}: {v:?}", stringify!($name))))
        };
    }

    v_stack! {
        spacing = 5;
        margin = 10;
        align = Alignment::TOP_LEFT;
        background_color = rgb(0.1, 0.1, 0.1);
        font_family = "monospace";
        opacity = 80.pct();
        items = widgets![
            status!(actual_position),
            status!(actual_size),
            status!(restore_state),
            status!(restore_rect),
        ]
    }
}
