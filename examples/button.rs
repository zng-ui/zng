#![recursion_limit = "256"]

#[macro_use]
extern crate zero_ui;
#[macro_use]
extern crate enclose;

use zero_ui::{core::var::Var, prelude::*};

fn main() {
    better_panic::install();

    App::default().run(|ctx| {
        ctx.services.req::<Windows>().open(|_| {
            let size = var((800., 600.));
            let title = size.map(|s: &LayoutSize| formatx!("Button Example - {}x{}", s.width.ceil(), s.height.ceil()));
            window! {
                background_color: rgb(0.1, 0.1, 0.1);
                size: size;
                title: title;
                => example()
            }
        });
    })
}

fn example() -> impl UiNode {
    //let t = var("Click Me!");
    let t_color = var(rgb(0, 100, 200));
    let content_align = var(Alignment::CENTER);
    let size = var((300.0, 200.0));
    let is_state = var(false);
    let t = is_state.map(|b| formatx!("is_pressed: {}", b));

    button! {
        on_click: enclose!{ (t, t_color, content_align, size) move |a| {
            let u = &mut a.ctx().updates;
            u.push_set(&t, "Clicked!".to_text()).ok();
            u.push_set(&t_color, rgb(100, 50, 200)).ok();
            u.push_set(&content_align, Alignment::BOTTOM_LEFT).ok();
            u.push_set(&size, LayoutSize::new(400.0, 100.0)).ok();
        }};
        is_pressed: is_state;
        content_align: content_align;
        size: size;
        align: Alignment::CENTER;
        font_size: 28;
        text_color: t_color;
        margin: 10.0;
        => {
            text(t)
        }
    }
}
