#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::{
    color::{filter::opacity, gradient::stops},
    layout::size,
    prelude::*,
};
use zng_app::view_process::VIEW_PROCESS;
use zng_view::extensions::ViewExtensions;

fn main() {
    examples_util::print_info();
    zng_view::init_extended(test_extensions);

    APP.defaults().run_window(async {
        Window! {
            title = "View-Process Respawn Example";
            icon = WindowIcon::render(icon);
            start_position = window::StartPosition::CenterMonitor;
            widget::foreground = window_status();
            child_align = Align::CENTER;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 5;
                children_align = Align::TOP;
                children = ui_vec![
                    Text! {
                        txt = "The renderer and OS windows are created in another process, the `view-process`. \
                               It automatically respawns in case of a graphics driver crash or other similar fatal error.";
                        txt_align = Align::CENTER;
                        layout::max_width = 620;
                    },
                    respawn(),
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
    Button! {
        child = Text!("Respawn (F5)");
        gesture::click_shortcut = shortcut!(F5);
        on_click = hn!(|_| {
            VIEW_PROCESS.respawn();
        });
    }
}

fn crash_respawn() -> impl UiNode {
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
        layout::align = Align::TOP_LEFT;
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
    ext.command::<(), ()>("zng.examples.respawn.crash", |_, _| panic!("CRASH"));
    ext
}
