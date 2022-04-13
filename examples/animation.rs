#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-animation.json.gz", &[("example", &"animation")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|_| {
        let x = var_from(0);

        window! {
            title = "Animation Example";

            min_size = (800, 620);
            content_align = unset!;
            padding = 10;
            content = h_stack(widgets![
                v_stack! {
                    id = "menu";
                    spacing = 5;
                    items = widgets![
                        ease_btn(&x, "linear", easing::linear),
                        ease_btn(&x, "quad", easing::quad),
                        ease_btn(&x, "cubic", easing::cubic),
                        ease_btn(&x, "quart", easing::quart),
                        ease_btn(&x, "quint", easing::quint),
                        ease_btn(&x, "sine", easing::sine),
                        ease_btn(&x, "expo", easing::expo),
                        ease_btn(&x, "circ", easing::circ),
                        ease_btn(&x, "back", easing::back),
                        ease_btn(&x, "elastic", easing::elastic),
                        ease_btn(&x, "bounce", easing::bounce),
                        ease_btn(&x, "step_ceil", |t| easing::step_ceil(5, t)),
                        ease_btn(&x, "step_floor", |t| easing::step_floor(5, t)),
                        ease_btn(&x, "none", easing::none),
                    ]
                },
                container! {
                    id = "demo-area";
                    min_width = 500;
                    content_align = Align::LEFT;
                    margin = (0, 0, 0, 100);
                    content = blank! {
                        id = "ball";
                        size = (40, 40);
                        corner_radius = 20;
                        background_color = colors::RED;

                        x;
                    };
                    background = z_stack!{
                        items_align = Align::LEFT;
                        items = widgets![
                            marker("0", 0),
                            marker("50", 50),
                            marker("100", 100),
                            marker("150", 150),
                            marker("200", 200),
                            marker("250", 250),
                            marker("300", 300),
                        ]
                    }
                }
            ]);
        }
    })
}

fn ease_btn(l: &RcVar<Length>, name: impl Into<Text>, easing: impl Fn(EasingTime) -> EasingStep + Clone + 'static) -> impl Widget {
    button! {
        content = text(name.into());
        on_click = hn!(l, |ctx, _| {
            l.set_ease(ctx, 0, 300, 1.secs(), easing.clone());
        });
    }
}

fn marker(c: impl Into<Text>, x: impl Into<Length>) -> impl Widget {
    text! {
        text = c.into();            
        color = colors::WHITE.with_alpha(30.pct());
        font_size = 20;
        font_weight = FontWeight::BOLD;
        x = x.into();
    }
}