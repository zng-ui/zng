#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

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
            content = v_stack! {
                spacing = 10;
                items = widgets![
                    example("LayerIndex::TOP_MOST, LayerMode::DEFAULT", container! {
                        layer = LayerIndex::TOP_MOST, LayerMode::DEFAULT;
                        background_color = colors::RED.with_alpha(40.pct());
                        content = text! {
                            text = "Overlay!";
                            font_size = 32.pt();
                        };
                    }),
                    example("LayerIndex::ADORNER, LayerMode::OFFSET", container! {
                        layer = LayerIndex::ADORNER, LayerMode::OFFSET;
                        background_color = colors::RED.darken(40.pct());
                        padding = 4;
                        content = text("Adorner!");
                    }),                    
                    example("20, LayerMode::FILTER", container! {
                        layer = 20, LayerMode::FILTER;
                        background_color = colors::BLUE.darken(40.pct());
                        padding = 4;
                        content = text("Same Filter!");
                        size = (100, 70);
                    }),
                    example("10, LayerMode::ALL", container! {
                        layer = 10, LayerMode::ALL;
                        background_color = colors::BLUE.darken(40.pct());
                        padding = 4;
                        content = text("Same transform and filter!");
                    }),
                    example("0, LayerMode::DEFAULT", container! {
                        layer = 0, LayerMode::DEFAULT;
                        background_color = colors::BLUE.darken(40.pct());
                        padding = 4;
                        content = text("Sad!");
                    }),
                ];
            };
        }
    })
}

fn example(name: impl IntoVar<Text>, layered_wgt: impl Widget) -> impl Widget {
    let show = var(false);
    button! {
        transform = translate_y(-20).rotate((-3).deg()).skew_x(3.deg());
        filter = drop_shadow((1, 1), 2, colors::BLACK);

        background = container! {
            align = Alignment::TOP_LEFT;
            content = layered_wgt;
            visibility = show.map_into();
        };
        content = text(name.into_var());

        enabled = show.map(|b| !*b);
        on_click = async_hn!(show, |ctx, _| {
            show.set(&ctx, true);
            task::timeout(1.secs()).await;
            show.set(&ctx, false);
        });
    }
}
