//! Demonstrates borders, corner radius, multiple borders per widget and clip-to-bounds.

use zng::{
    prelude::*,
    widget::{background_color, border_align, corner_radius},
};

mod widgets;

fn main() {
    zng::env::init_res(concat!(env!("CARGO_MANIFEST_DIR"), "/res"));
    zng::env::init!();

    // zng::view_process::default::run_same_process(app_main);
    app_main();
}

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            title = "Border Example";

            background_color = web_colors::BLUE.darken(70.pct());

            color_scheme = color::ColorScheme::Dark;

            child = Stack! {
                layout::align = Align::CENTER;
                spacing = 20;
                direction = StackDirection::left_to_right();
                children = ui_vec![
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 20;
                        children = ui_vec![
                            widgets::MrBorders! {
                                border_align = 0.pct();
                                child = Text!("border_align = 0.pct();");
                            },
                            widgets::MrBorders! {
                                border_align = (1.0 / 3.0).fct();
                                child = Text!("border_align = (1.0 / 3.0).fct();");
                            },
                            widgets::MrBorders! {
                                border_align = 50.pct();
                                child = Text!("border_align = 50.pct();");
                            },
                            widgets::MrBorders! {
                                border_align = 100.pct();
                                child = Text!("border_align = 100.pct();");
                            },
                            Button! {
                                child = Text!("border_img");
                                on_click = hn!(|_| on_border_img());
                                zng::color::base_color = web_colors::GREEN.darken(40.pct());
                                zng::mouse::cursor = zng::mouse::CursorIcon::Pointer;

                                widget::border_img = {
                                    widths: 5,
                                    source: zng::env::res("border.png"),
                                    slices: (100.0 / 3.0).pct(),
                                };
                            }
                        ];
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 20;
                        children = ui_vec![
                            widgets::MrBorders! {
                                child = Text!("corner_radius = 0;");
                                corner_radius = 0;
                            },
                            widgets::MrBorders! {
                                child = Text!("corner_radius = 40;");
                                corner_radius = 40;
                            },
                            widgets::MrBorders! {
                                border_align = 100.pct();
                                child = widgets::MrBorders! {
                                    border_align = 100.pct();
                                    child = widgets::MrBorders! {
                                        border_align = 100.pct();
                                        child = Text!("Nested");
                                    };
                                };
                            },
                            clip_to_bounds_demo(),
                        ];
                    },
                ];
            };
        }
    })
}

fn on_border_img() {
    WINDOWS.focus_or_open("border_img-win", async {
        let fill = var(false);
        let repeat = var(zng::widget::BorderRepeats::default());
        Window! {
            title = "border_img";
            child = Container! {
                widget::border_img = {
                    widths: 15,
                    source: zng::env::res("border-test.png"),
                    slices: (100.0 / 3.0).pct(),
                };
                widget::border_img_fill = fill.clone();
                widget::border_img_repeat = repeat.map_into();

                widget::foreground_highlight = -15, 15, colors::RED.with_alpha(30.pct());

                layout::margin = 20;
                padding = 20;
                child = Stack!(
                    top_to_bottom,
                    ui_vec![
                        Toggle! {
                            checked = fill;
                            style_fn = toggle::CheckStyle!();
                            child = Text!("border_img_fill = true;");
                        },
                        Text!(txt = "border_img_repeat = "; layout::margin = (25, 0, 0, 0)),
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 5;
                            toggle::selector = toggle::Selector::single(repeat);
                            layout::margin = (0, 0, 0, 50);
                            children = {
                                use zng::widget::RepeatMode::*;
                                [
                                    (Stretch, Stretch),
                                    (Repeat, Repeat),
                                    (Round, Round),
                                    (Space, Space),
                                    (Stretch, Space),
                                    (Space, Stretch),
                                ]
                                .into_iter()
                                .map(|m| {
                                    Toggle! {
                                        value::<zng::widget::BorderRepeats> = m;
                                        child = if m.0 == m.1 { Text!("{:?};", m.0) } else { Text!("{m:?};") };
                                        style_fn = toggle::RadioStyle!();
                                    }
                                })
                            };
                        }
                    ]
                );
            };
        }
    });
}

fn clip_to_bounds_demo() -> UiNode {
    let clip = var(true);
    Container! {
        child_align = Align::FILL;
        corner_radius = 10;
        widget::border = 0.5, web_colors::RED.darken(20.pct());
        clip_to_bounds = clip.clone();
        gesture::on_click = hn!(clip, |_| clip.modify(|c| **c = !**c));
        child = Text! {
            corner_radius = 0;
            background_color = web_colors::GREEN.darken(40.pct());
            layout::padding = 3;
            layout::rotate = -(5.deg());
            txt_align = Align::CENTER;
            txt = clip.map(|c| formatx!("clip_to_bounds = {c}"));
        };
    }
}
