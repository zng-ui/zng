use crate::{FPS, FROM_COLOR, TO_COLOR};
use easing::EasingTime;
use std::collections::HashMap;
use window::RenderMode;
use zng::{
    image::{self, ImageSource},
    prelude::*,
    prelude_wgt::*,
    var::animation::{self, easing::EasingFn},
};

pub(crate) fn ease_btn(l: &Var<Length>, color: &Var<Rgba>, name: &'static str, easing: EasingFn, easing_mod: &Var<Txt>) -> UiNode {
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
                    img_loading_fn = wgt_fn!(|_| Wgt! {
                        layout::size = (64, 64);
                        layout::margin = 10;
                    });
                    layout::size = 64 + 10;
                    source = easing.map(move |(name, f)| plot_cache.entry(name.clone()).or_insert_with(|| plot(f.clone())).clone());
                },
            ];
        };
        on_click = hn!(l, color, |_| {
            // ANIMATION
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

                children.push(Wgt! {
                    layout::offset = (x, y);
                    layout::size = (3, 3);
                    widget::corner_radius = 2;
                    layout::translate = -1.5, -1.5;
                    widget::background_color = color_t.sample(y_fct);
                })
            }

            image::IMAGE_RENDER.retain().set(true);
            let meta_color = WINDOW.vars().actual_color_scheme().map(|t| match t {
                ColorScheme::Dark => rgba(255, 255, 255, 0.4),
                ColorScheme::Light | _ => rgba(0, 0, 0, 0.4),
            });

            children.push(Text! {
                txt = "v";
                font_size = 12;
                font_style = FontStyle::Italic;
                font_color = meta_color.clone();
                layout::offset = (-3.dip() - 100.pct(), -3.dip());
            });
            children.push(Text! {
                txt = "t";
                font_size = 12;
                font_style = FontStyle::Italic;
                font_color = meta_color.clone();
                layout::offset = (size.0.dip() - 100.pct() - 3.dip(), size.1 - 3);
            });
            Stack! {
                children_align = Align::TOP_LEFT;
                children;
                layout::size;
                widget::border = (0, 0, 1, 1), meta_color.map_into();
                layout::margin = 10;
            }
        }),
    )
}

pub(crate) fn ruler() -> UiNode {
    Stack! {
        children_align = Align::LEFT;
        children = (0..=300).step_by(10).map(|x| {
            zng::rule_line::RuleLine! {
                orientation = LineOrientation::Vertical;
                color = text::FONT_COLOR_VAR.map(|c| c.with_alpha(40.pct()));
                layout::x = x.dip();
                layout::height = if x % 100 == 0 {
                    52
                } else if x % 50 == 0 {
                    22
                } else {
                    12
                };
            }
        });
    }
}
