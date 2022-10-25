#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("transform");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Transform Example";
            child_align = Align::CENTER;
            child = h_stack! {
                spacing = 40;
                children = ui_list![
                    v_stack! {
                        spacing = 25;
                        children_align = Align::TOP;
                        children = ui_list![
                            transformed("Translate -10", translate(-10, -10)),
                            transformed_at("Rotate 10º (0, 0)", rotate(10.deg()), (0, 0)),
                            transformed("Rotate 10º", rotate(10.deg())),
                            transformed("Skew-X 15º", skew_x(15.deg())),
                            transformed("Scale 130%", scale(130.pct())),
                            transformed("Identity", Transform::identity()),
                        ];
                    },
                    v_stack! {
                        spacing = 40;
                        children = ui_list![
                            transform_stack(),
                            transform_order(),
                        ]
                    }
                ]
            };
        }
    })
}

fn transformed(label: impl Into<Text>, transform: Transform) -> impl UiNode {
    container! {
        child = container! {
            transform;
            child = text(label.into());
            background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
            padding = 10;
        };
        border = 2, (colors::GRAY, BorderStyle::Dashed);
    }
}
fn transformed_at(label: impl Into<Text>, transform: Transform, origin: impl Into<Point>) -> impl UiNode {
    container! {
        child = container! {
            transform;
            transform_origin = origin.into();
            child = text(label.into());
            background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
            padding = 10;
        };
        border = 2, (colors::GRAY, BorderStyle::Dashed);
    }
}

fn transform_stack() -> impl UiNode {
    // the panel widget uses its child transform to position the widget for performance reasons,
    // the widget transform does not affect.
    v_stack! {
        spacing = 5;
        children = ui_list![
            container! {
                child = text("Identity");
                background_color = colors::DARK_GRAY.with_alpha(80.pct());
                padding = 10;
            },
            container! {
                id = "in-stack";
                transform = rotate(45.deg());
                child = text("Rotated 45º");
                background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
                padding = 10;

                when *#is_hovered {
                    z_index = ZIndex::DEFAULT + 1;
                }
            },
            container! {
                child = text("Identity");
                background_color = colors::DARK_GRAY.with_alpha(80.pct());
                padding = 10;
            },
        ];
    }
}

fn transform_order() -> impl UiNode {
    // transform created using a single property or two properties generate the same transform because
    // are in the same order.
    z_stack(ui_list![
        blank! {
            // single property
            transform = rotate(10.deg()).translate(50, 30);

            size = (60, 60);
            background_color = colors::BLUE.lighten(50.pct());

            when *#is_hovered {
                z_index = ZIndex::DEFAULT + 1;
            }
        },
        blank! {
            // two properties
            rotate = 10.deg();
            translate = 50, 30;

            size = (60, 60);
            background_color = colors::GREEN;

            when *#is_hovered {
                z_index = ZIndex::DEFAULT - 1;
            }
        },
    ])
}
