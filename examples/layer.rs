#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::{
    core::widget_base::hit_testable,
    properties::events::widget::on_pre_init,
    widgets::window::{AnchorMode, LayerIndex, WindowLayers},
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
                WindowLayers::insert(ctx, LayerIndex::TOP_MOST - 100, text! {
                    hit_testable = false;
                    text = "INITED";
                    font_size = 150;
                    opacity = 3.pct();
                    // rotate = 45.deg();
                    align = Alignment::CENTER;
                })
            });

            content = v_stack! {
                spacing = 5;
                items = widgets![
                    overlay_btn(),

                    layer_n_btn(7, colors::DARK_GREEN),
                    layer_n_btn(8, colors::DARK_BLUE),
                    layer_n_btn(9, colors::DARK_RED),
                ];
            };
        }
    })
}

fn overlay_btn() -> impl Widget {
    button! {
        content = text("TOP_MOST");
        on_click = hn!(|ctx, _| {
            WindowLayers::insert(ctx, LayerIndex::TOP_MOST, overlay());
        });
    }
}
fn overlay() -> impl Widget {
    container! {
        id = "overlay";
        modal = true;
        background_color = colors::GRAY.with_alpha(40.pct());
        content = container! {
            focus_scope = true;
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

fn layer_n_btn(n: u32, color: Rgba) -> impl Widget {
    let label = formatx!("Layer {n}");
    button! {
        content = text(label.clone());
        on_click = async_hn!(label, |ctx, _| {
            let id = WidgetId::new_unique();
            ctx.with(|ctx| WindowLayers::insert(ctx, n, container! {
                id;
                content = text! {
                    text = label.clone();
                    font_size = 16;
                    font_weight = FontWeight::BOLD;
                };
                background_color = color.with_alpha(80.pct());
                padding = 10;
                margin = {
                    let inc = n as i32 * 10;
                    (60 + inc, 10, 0, inc - 40)
                };
                align = Alignment::TOP;
                hit_testable = false;
            }));

            task::timeout(2.secs()).await;

            ctx.with(|ctx| WindowLayers::remove(ctx, id));
        });
    }
}
