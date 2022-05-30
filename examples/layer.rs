#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;
use zero_ui::{
    properties::events::{
        mouse::{on_mouse_enter, on_mouse_leave},
        widget::on_pre_init,
    },
    widgets::window::{AnchorMode, AnchorSize, AnchorTransform, LayerIndex, WindowLayers},
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

            // zero_ui::widgets::inspector::show_bounds = true;

            // you can use the pre-init to insert layered widgets
            // before the first render.
            on_pre_init = hn!(|ctx, _| {
                WindowLayers::insert(ctx, LayerIndex::TOP_MOST - 100, text! {
                    hit_test_mode = HitTestMode::Disabled;
                    text = "on_pre_init";
                    font_size = 72;
                    font_family = "monospace";
                    opacity = 3.pct();
                    // rotate = 45.deg();
                    align = Align::CENTER;
                })
            });

            content_align = Align::CENTER;
            content = v_stack! {
                spacing = 5;
                items = widgets![
                    overlay_example(),
                    layer_index_example(),
                    anchor_example(),
                    transform_anchor_example(),
                ];
            };
        }
    })
}

fn overlay_example() -> impl Widget {
    button! {
        content = text("TOP_MOST");
        on_click = hn!(|ctx, _| {
            WindowLayers::insert(ctx, LayerIndex::TOP_MOST, overlay("overlay", 0));
        });
    }
}
fn overlay(id: impl Into<WidgetId>, offset: i32) -> impl Widget {
    let id = id.into();
    container! {
        id;
        modal = true;
        background_color = colors::GRAY.with_alpha(10.pct());
        content_align = Align::CENTER;
        content = container! {
            offset = (offset, offset);
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            background_color = colors::GREEN.darken(80.pct());
            button::theme::corner_radius = 0;
            padding = 2;
            content = v_stack! {
                items_align = Align::RIGHT;
                items = widgets![
                    text! {
                        text = "Overlay inserted in the TOP_MOST layer.";
                        margin = 15;
                    },
                    h_stack! {
                        spacing = 2;
                        items = widgets![
                            button! {
                                visibility = offset < 50;
                                content = text("Open Another");
                                on_click = hn!(|ctx, _| {
                                    WindowLayers::insert(ctx, LayerIndex::TOP_MOST, overlay(WidgetId::new_unique(), offset + 10));
                                })
                            },
                            button! {
                                content = text("Remove");
                                on_click = hn!(|ctx, _| {
                                    WindowLayers::remove(ctx, id);
                                })
                            },
                        ]
                    }
                ]
            }
        }
    }
}

fn layer_index_example() -> impl Widget {
    // demonstrates that the z-order is not affected by the order of insertion.
    h_stack! {
        spacing = 5;
        items = widgets![
            layer_n_btn(7, colors::DARK_GREEN),
            layer_n_btn(8, colors::DARK_BLUE),
            layer_n_btn(9, colors::DARK_RED),
        ]
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
                    (20 + inc, 10, 0, inc - 40)
                };
                align = Align::TOP;
                hit_test_mode = HitTestMode::Disabled;
            }));

            task::timeout(2.secs()).await;

            ctx.with(|ctx| WindowLayers::remove(ctx, id));
        });
    }
}

fn anchor_example() -> impl Widget {
    let points = [
        Point::top_left(),
        Point::top(),
        Point::top_right(),
        Point::right(),
        Point::bottom_right(),
        Point::bottom(),
        Point::bottom_left(),
        Point::left(),
    ];
    let points_len = points.len();
    let point_index = var(0);
    let point = point_index.map(move |&i| points[i].clone());

    let anchor_mode = point.map(move |p| AnchorMode {
        transform: AnchorTransform::InnerOffset(p.clone()),
        size: AnchorSize::Unbounded,
        visibility: true,
        interaction: false,
        corner_radius: false,
    });

    let next_point = hn!(|ctx, _| {
        point_index.modify(ctx, move |mut i| {
            let next = *i + 1;
            *i = if next == points_len { 0 } else { next };
        })
    });

    button! {
        id = "anchor";
        content = text("Anchored");

        margin = (60, 0);
        align = Align::CENTER;

        on_mouse_enter = hn!(|ctx, _| {
            WindowLayers::insert_anchored(ctx, LayerIndex::ADORNER, "anchor", anchor_mode.clone(), text! {
                id = "anchored";
                text = "Example";
                padding = 4;
                font_weight = FontWeight::BOLD;
                background_color = colors::GREEN.darken(40.pct());
                border = 1, colors::GREEN.darken(20.pct());
                offset = point.map(|p|p.clone().as_vector() - Vector::splat(100.pct()));
                margin = 2;
                hit_test_mode = HitTestMode::Disabled;
            })
        });
        on_mouse_leave = hn!(|ctx, _| {
            WindowLayers::remove(ctx, "anchored");
        });

        on_click = next_point;
    }
}

fn transform_anchor_example() -> impl Widget {
    let mut insert = true;
    button! {
        id = "t-anchor";
        content = text("Transform Anchored");

        rotate = 20.deg();
        scale = 110.pct();

        on_click = hn!(|ctx, _| {
            if insert {
                WindowLayers::insert_anchored(ctx, LayerIndex::ADORNER, "t-anchor", AnchorMode::foreground(), container! {
                    id = "t-anchored";
                    content_align = Align::TOP_LEFT;
                    border = 1, colors::GREEN.lighten(30.pct());
                    hit_test_mode = HitTestMode::Disabled;
                    content = text! {
                        y = -(2.dip() + 100.pct());
                        text = "example";
                        font_weight = FontWeight::BOLD;
                    }
                })
            } else {
                WindowLayers::remove(ctx, "t-anchored");
            }
            insert = !insert;
        })
    }
}
