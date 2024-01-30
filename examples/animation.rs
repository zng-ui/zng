#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashMap;

use zero_ui::{
    button,
    color::{ColorScheme, Rgba},
    image,
    layout::{margin, offset, size},
    prelude::*,
    rule_line::RuleLine,
    var::{
        animation::{
            self,
            easing::{EasingFn, EasingTime},
        },
        ArcVar, VARS,
    },
    widget::{background_color, corner_radius, LineOrientation},
    window::RenderMode,
};

use zero_ui::view_process::prebuilt as view_process;

fn main() {
    examples_util::print_info();
    view_process::init();

    // let rec = examples_util::record_profile("animation");

    // view_process::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            title = "Animation Example";
            padding = 10;
            child_align = Align::CENTER;
            child = example();
        }
    })
}

const FROM_COLOR: Rgba = web_colors::RED;
const TO_COLOR: Rgba = web_colors::GREEN;
const FPS: u32 = 60;

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
                    let mode = |m: Txt| Toggle! {
                        child = Text!(m.clone());
                        value::<Txt> = m;
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
                button::style_fn = Style! { layout::padding = 3 };
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
                    ease_btn(&x, &color, "step_ceil(6)", EasingFn::custom(|t| easing::step_ceil(6, t)), &easing_mod),
                    ease_btn(&x, &color, "step_floor(6)", EasingFn::custom(|t| easing::step_floor(6, t)), &easing_mod),
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

fn ease_btn(l: &ArcVar<Length>, color: &ArcVar<Rgba>, name: &'static str, easing: EasingFn, easing_mod: &ArcVar<Txt>) -> impl UiNode {
    let name = if name.is_empty() { formatx!("{easing:?}") } else { name.to_txt() };
    let easing = easing_mod.map(clmv!(easing, |m| {
        let f = match m.as_str() {
            "ease_in" => easing.clone(),
            "ease_out" => easing.clone().ease_out(),
            "ease_in_out" => easing.clone().ease_in_out(),
            "ease_out_in" => easing.clone().ease_out_in(),
            "reverse" => easing.clone().reverse(),
            "reverse_out" => easing.clone().reverse_out(),
            _ => unreachable!(),
        };
        (m.clone(), f)
    }));
    let mut plot_cache = HashMap::new(); // to reuse rendered images
    Button! {
        grid::cell::at = grid::cell::AT_AUTO;
        child = Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 2;
            children_align = Align::TOP;
            children = ui_vec![
                Text!(name),
                Image! {
                    img_scale_ppi = true;
                    img_loading_fn = wgt_fn!(|_| Wgt! {
                        size = (64, 64);
                        margin = 10;
                    });
                    source = easing.map(move |(name, f)| plot_cache.entry(name.clone()).or_insert_with(|| plot(f.clone())).clone());
                },
            ]
        };
        on_click = hn!(l, color, |_| {
            let f = easing.get().1;
            l.set_ease(0, 300, 1.secs(), f.ease_fn()).perm();
            color.set_ease(FROM_COLOR, TO_COLOR, 1.secs(), f.ease_fn()).perm();
        });
    }
}
fn plot(easing: EasingFn) -> ImageSource {
    let size = (64, 64);
    ImageSource::render_node(
        RenderMode::Software,
        clmv!(size, |_| {
            let mut children = ui_vec![];
            let color_t = animation::Transition::new(FROM_COLOR, TO_COLOR);
            let fps_f = FPS as f32;
            for i in 0..=FPS {
                let x_fct = (i as f32 / fps_f).fct();
                let x = size.0 * x_fct;

                let y_fct = easing(EasingTime::new(x_fct));
                let y = size.1 * (1.fct() - y_fct);

                children.push(
                    Wgt! {
                        offset = (x, y);
                        size = (3, 3);
                        corner_radius = 2;
                        layout::translate = -1.5, -1.5;
                        background_color = color_t.sample(y_fct);
                    }
                    .boxed(),
                )
            }

            image::IMAGE_RENDER.retain().set(true);
            let meta_color = WINDOW.vars().actual_color_scheme().map(|t| match t {
                ColorScheme::Light => rgba(0, 0, 0, 0.4),
                ColorScheme::Dark => rgba(255, 255, 255, 0.4),
            });

            #[allow(clippy::precedence)]
            children.push(
                Text! {
                    txt = "v";
                    font_size = 12;
                    font_style = FontStyle::Italic;
                    font_color = meta_color.clone();
                    offset = (-3.dip() - 100.pct(), -3.dip());
                }
                .boxed(),
            );
            children.push(
                Text! {
                    txt = "t";
                    font_size = 12;
                    font_style = FontStyle::Italic;
                    font_color = meta_color.clone();
                    offset = (size.0.dip() - 100.pct() - 3.dip(), size.1 - 3);
                }
                .boxed(),
            );
            Stack! {
                children_align = Align::TOP_LEFT;
                children;
                size;
                widget::border = (0, 0, 1, 1), meta_color.map_into();
                margin = 10;
            }
        }),
    )
}

fn ruler() -> impl UiNode {
    Stack! {
        children_align = Align::LEFT;
        children = (0..=300).step_by(10)
            .map(|x| RuleLine! {
                orientation = LineOrientation::Vertical;
                color = text::FONT_COLOR_VAR.map(|c| c.with_alpha(40.pct()));
                layout::x = x.dip();
                layout::height = if x % 100 == 0 { 52 } else if x % 50 == 0 { 22 } else { 12 };
            }
            .boxed())
            .collect::<Vec<_>>(),
    }
}
