//! Demonstrates animation, easing functions.

use zng::{
    button,
    color::Rgba,
    layout::{margin, size},
    prelude::*,
    var::VARS,
    var::animation::easing::EasingFn,
    widget::{background_color, corner_radius},
};

mod widgets;
use widgets::{ease_btn, ruler};

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        Window! {
            title = "Animation Example";
            padding = 10;
            child_align = Align::CENTER;
            child = example();
        }
    })
}

pub(crate) const FROM_COLOR: Rgba = web_colors::RED;
pub(crate) const TO_COLOR: Rgba = web_colors::GREEN;
pub(crate) const FPS: u32 = 60;

fn example() -> impl UiNode {
    // VARS.animation_time_scale().set(0.5.fct());
    VARS.frame_duration().set((1.0 / FPS as f32).secs());

    let x = var(0.dip());
    let color = var(FROM_COLOR);

    // x.trace_value(move |v| {
    //     tracing::debug_span!("x", value = ?v, thread = "<x>").entered()
    // })
    // .perm();

    let easing_mod = var(Txt::from("ease_out"));

    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 10;
        children_align = Align::TOP;
        children = ui_vec![
            Container! {
                id = "demo";
                layout::width = 301;
                widget::background = ruler();
                margin = (0, 0, 40, 0);
                child_align = Align::LEFT;
                child = Wgt! {
                    id = "ball";
                    size = (40, 40);
                    corner_radius = 20;
                    background_color = color.clone();

                    layout::x = x.map(|x| x.clone() - 20.dip());

                    when *#gesture::is_hovered {
                        background_color = web_colors::LIME;
                    }
                };
            },
            Stack! {
                id = "mod-menu";
                direction = StackDirection::left_to_right();
                spacing = 2;
                toggle::selector = toggle::Selector::single(easing_mod.clone());
                children = {
                    let mode = |m: Txt| {
                        Toggle! {
                            child = Text!(m.clone());
                            value::<Txt> = m;
                        }
                    };
                    ui_vec![
                        mode(Txt::from("ease_in")),
                        mode(Txt::from("ease_out")),
                        mode(Txt::from("ease_in_out")),
                        mode(Txt::from("ease_out_in")),
                        mode(Txt::from("reverse")),
                        mode(Txt::from("reverse_out")),
                    ]
                }
            },
            Grid! {
                id = "easing-menu";
                spacing = 2;
                columns = ui_vec![grid::Column!(1.lft()); 7];
                auto_grow_fn = wgt_fn!(|_| grid::Row!(1.lft()));
                button::style_fn = Style! {
                    layout::padding = 3
                };
                cells = ui_vec![
                    ease_btn(&x, &color, "", EasingFn::Linear, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Quad, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Cubic, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Quart, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Quint, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Sine, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Expo, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Circ, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Back, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Elastic, &easing_mod),
                    ease_btn(&x, &color, "", EasingFn::Bounce, &easing_mod),
                    ease_btn(
                        &x,
                        &color,
                        "step_ceil(6)",
                        EasingFn::custom(|t| easing::step_ceil(6, t)),
                        &easing_mod
                    ),
                    ease_btn(
                        &x,
                        &color,
                        "step_floor(6)",
                        EasingFn::custom(|t| easing::step_floor(6, t)),
                        &easing_mod
                    ),
                    ease_btn(&x, &color, "", EasingFn::None, &easing_mod),
                ]
            },
            Button! {
                child = Text!("reset");
                widget::foreground_highlight = {
                    offsets: -2,
                    widths: 1,
                    sides: web_colors::DARK_RED,
                };
                gesture::click_shortcut = shortcut![Escape];
                on_click = hn!(x, color, |_| {
                    x.set(0);
                    color.set(FROM_COLOR);
                });
            },
        ]
    }
}
