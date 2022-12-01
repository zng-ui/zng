#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::app::view_process::ViewProcess;
use zero_ui::prelude::*;

// use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();
    App::default().run_window(|ctx| {
        window! {
            title = "View-Process Respawn Example";
            icon = WindowIcon::render(|_| icon());
            start_position = StartPosition::CenterMonitor;
            on_key_down = hn!(|ctx, args: &KeyInputArgs| {
                if args.key == Some(Key::F5) {
                    ViewProcess::req(ctx.services).respawn();
                }
            });
            foreground = window_status(ctx);
            child_align = Align::CENTER;
            child = v_stack! {
                spacing = 5;
                children_align = Align::TOP;
                children = ui_list![
                    text! {
                        txt = "The renderer and OS windows are created in another process, the `view-process`. \
                               It automatically respawns in case of a graphics driver crash or other similar fatal error.";
                        txt_align = Align::CENTER;
                        max_width = 620;
                    },
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

fn respawn() -> impl UiNode {
    button! {
        child = text("Respawn (F5)");
        on_click = hn!(|ctx, _| {
            ViewProcess::req(ctx.services).respawn();
        });
    }
}

#[cfg(debug_assertions)]
fn crash_respawn() -> impl UiNode {
    button! {
        child = text("Crash View-Process");
        on_click = hn!(|ctx, _| {
            ViewProcess::req(ctx.services).crash_view_process();
        });
    }
}

fn click_counter() -> impl UiNode {
    let t = var_from("Click Me!");
    let mut count = 0;

    button! {
        on_click = hn!(t, |ctx, _| {
            count += 1;
            let new_txt = formatx!("Clicked {count} time{}!", if count > 1 {"s"} else {""});
            t.set(ctx, new_txt);
        });
        child = text(t);
    }
}

fn image() -> impl UiNode {
    v_stack! {
        spacing = 3;
        children = ui_list![
            strong("Image:"),
            image! { source = "examples/res/window/icon-bytes.png"; size = (32, 32); },
        ];
    }
}

fn window_status(ctx: &mut WindowContext) -> impl UiNode {
    let vars = WindowVars::req(ctx);

    macro_rules! status {
        ($name:ident) => {
            text(vars.$name().map(|v| formatx!("{}: {v:?}", stringify!($name))))
        };
    }

    v_stack! {
        spacing = 5;
        margin = 10;
        align = Align::TOP_LEFT;
        background_color = color_scheme_map(colors::WHITE.with_alpha(10.pct()), colors::BLACK.with_alpha(10.pct()));
        font_family = "monospace";
        opacity = 80.pct();
        children = ui_list![
            status!(actual_position),
            status!(actual_size),
            status!(restore_state),
            status!(restore_rect),
        ]
    }
}

fn icon() -> impl UiNode {
    container! {
        size = (36, 36);
        background_gradient = Line::to_bottom_right(), stops![colors::ORANGE_RED, 70.pct(), colors::DARK_RED];
        corner_radius = 6;
        font_size = 28;
        font_weight = FontWeight::BOLD;
        child_align = Align::CENTER;
        child = text("R");
    }
}
