#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("profile-border.json.gz", &[("example", &"border")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Border Example";

            background_color = colors::RED.darken(70.pct());

            content = v_stack! {
                spacing = 20;
                items = widgets![
                    widgets::mr_borders! {
                        border_align = 0.pct();
                        content = text("border_align = 0.pct();");
                    },
                    widgets::mr_borders! {
                        border_align = 100.pct();
                        content = text("border_align = 100.pct();");
                    }
                ]
            };
        }
    })
}

mod widgets {
    use zero_ui::prelude::new_widget::*;

    #[widget($crate::widgets::mr_borders)]
    pub mod mr_borders {
        use super::*;

        inherit!(container);

        properties! {
            child { padding = 20; }

            background_color = colors::GREEN.darken(40.pct());

            border as border0 = 4, colors::WHITE.with_alpha(20.pct());
            border as border1 = 4, colors::BLACK.with_alpha(20.pct());
            border as border2 = 4, colors::WHITE.with_alpha(20.pct());

            corner_radius = 20;
        }
    }
}
