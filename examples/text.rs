#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Text Example";
            content: h_stack! {
                spacing: 40.0;
                items: ui_vec![
                    line_height(),
                    //text("Basic Text"),
                    //strong("Strong Text"),
                    //em("Emphasis Text"),
                    //colored_text()
                ];
            };
        }
    })
}

fn colored_text() -> impl Widget {
    text! {
        color: web_colors::LIGHT_GREEN;
        text: "Colored Text";
    }
}

fn section(header: &'static str, mut items: UiVec) -> impl Widget {
    items.insert(
        0,
        text! {
            text: header;
            font_weight: FontWeight::BOLD;
            margin: (0.0, 4.0);
        }
        .boxed(),
    );
    v_stack! {
        spacing: 5.0;
        items;
    }
}

fn line_height() -> impl Widget {
    section(
        "line_height",
        ui_vec![
            text! {
                text: "Default: 'Émp Giga Ç'";
                background_color: web_colors::LIGHT_BLUE;
                color: web_colors::BLACK;
            },
            text! {
                text: "1.3em: 'Émp Giga Ç'";
                background_color: web_colors::LIGHT_BLUE;
                color: web_colors::BLACK;
                //line_height: 1.3.em();
            }
        ],
    )
}
