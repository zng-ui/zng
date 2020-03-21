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
    let is_hovered = var(false);
    let is_pressed = var(false);
    let t = merge_var!(is_hovered.clone(), is_pressed.clone(), |a, b| formatx!("{} : {}", a, b));

    button! {
        on_click: enclose!{ (t_color) move |a| {
            let u = &mut a.ctx().updates;
            u.push_set(&t_color, rgb(100, 50, 200)).ok();
        }};
        is_hovered: is_hovered;
        is_pressed: is_pressed;
        size: (300.0, 200.0);
        align: Alignment::CENTER;
        font_size: 28;
        text_color: t_color;
        margin: 10.0;
        => {
            text(t)
        }
    }
}
