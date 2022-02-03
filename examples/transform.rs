#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-transform.json.gz", &[("example", &"transform")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Transform Example";
            content = v_stack! {
                spacing = 25;
                items_align = Alignment::TOP;
                items = widgets![
                    transformed("Translate -10", translate(-10, -10)),
                    transformed_at("Rotate 10º (0, 0)", rotate(10.deg()), (0, 0)),
                    transformed("Rotate 10º", rotate(10.deg())),
                    transformed("Skew-X 15º", skew_x(15.deg())),
                    transformed("Scale 130%", scale(130.pct())),
                    transformed("Identity", Transform::identity()),
                    transform_stack(),
                ];
            };
        }
    })
}

fn transformed(label: impl Into<Text>, transform: Transform) -> impl Widget {
    container! {
        content = container! {
            transform;
            content = text(label.into());
            background_color = colors::BROWN.with_alpha(80.pct());
            padding = 10;
        };
        border = 2, (colors::GRAY, BorderStyle::Dashed), 0;
    }
}
fn transformed_at(label: impl Into<Text>, transform: Transform, origin: impl Into<Point>) -> impl Widget {
    container! {
        content = container! {
            transform;
            transform_origin = origin.into();
            content = text(label.into());
            background_color = colors::BROWN.with_alpha(80.pct());
            padding = 10;
        };
        border = 2, (colors::GRAY, BorderStyle::Dashed), 0;
    }
}

fn transform_stack() -> impl Widget {
    v_stack! {
        spacing = 5;
        items = widgets![
            container! {
                content = text("normal");
                background_color = colors::GREEN.with_alpha(80.pct());
                padding = 10;
            },
            container! {
                id = "in-stack";
                transform = rotate(90.deg());
                content = text("rotated");
                background_color = colors::BROWN.with_alpha(80.pct());
                padding = 10;
            },
            container! {
                content = container! {
                    transform = rotate(90.deg());
                    content = text("rotated");
                    background_color = colors::BROWN.with_alpha(80.pct());
                    padding = 10;
                }
            }
        ];
    }
}
