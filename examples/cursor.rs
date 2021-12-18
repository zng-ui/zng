#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    // zero_ui_view::run_same_process(app_main);

    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        let mut demos = widget_vec![];
        for icon in CursorIcon::ALL {
            demos.push(cursor_demo(Some(*icon)));
        }

        window! {
            title = "Cursor Example";
            resizable = false;
            auto_size = true;
            padding = 20;
            content = v_stack(widgets![
                uniform_grid! {
                    columns = 5;
                    items = demos;
                },
                center(cursor_demo(None)),
            ])
        }
    })
}

fn cursor_demo(icon: Option<CursorIcon>) -> impl Widget {
    container! {
        cursor = icon;

        size = (150, 80);
        
        margin = 1;
        background_color = rgb(33, 33, 33);

        text_color = rgb(140, 140, 140);

        when self.is_hovered {
            text_color = colors::WHITE;
        }

        content_align = Alignment::TOP_LEFT;
        padding = (2, 5);

        content = text! {
            text = match icon {
                Some(ico) => formatx!("{:?}", ico),
                None => Text::from_static("<none>"),
            };

            font_style = match icon {
                Some(_) => FontStyle::Normal,
                None => FontStyle::Italic,
            };

            font_family = "monospace";
            font_size = 16;
            font_weight = FontWeight::BOLD;
        };
    }
}
