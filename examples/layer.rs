#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::{
    widgets::window::{AnchorMode, LayerIndex, WindowLayers},
    properties::events::widget::on_pre_init
};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-layer.json.gz", &[("example", &"layer")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Layer Example";

            // you can use the pre-init to insert layered widgets 
            // before the first render.
            on_pre_init = hn!(|ctx, _| {
                WindowLayers::insert(ctx, 10, text! {
                    text = "LAYER 10";
                    font_size = 200;
                    opacity = 3.pct();
                    // rotate = 45.deg();
                    align = Alignment::CENTER;
                })
            });

            content = v_stack! {
                spacing = 10;
                items = widgets![
                    overlay_btn(),
                ];
            };
        }
    })
}

fn overlay_btn() -> impl Widget {
    fn overlay() -> impl Widget {
        container! {
            id = "overlay";
            background_color = colors::GRAY.with_alpha(40.pct());
            content = container! {
                background_color = colors::GRAY.darken(50.pct());
                padding = 2;
                content = v_stack! {
                    items_align = Alignment::RIGHT;
                    items = widgets![
                        text! {
                            text = "Overlay inserted in the TOP_MOST layer.";
                            margin = 15;
                        },
                        button! {
                            content = text("Ok");
                            on_click = hn!(|ctx, _| {
                                WindowLayers::remove(ctx, "overlay");
                            })
                        }
                    ]
                }
            }
        }
    }

    button! {
        content = text("Open Overlay");
        on_click = hn!(|ctx, _| {
            WindowLayers::insert(ctx,  LayerIndex::TOP_MOST, overlay());
        });
    }
}
