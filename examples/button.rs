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
                size: size;
                title: title;
                => example()
            }
        });
    })
}

fn example() -> impl UiNode {
    let t = var("Click Me!"); 

    button! {
        on_click: enclose!{ (t) move |a| {
            let ctx = a.ctx();
            ctx.updates.push_set(&t, "Clicked!".into(), ctx.vars).unwrap();
        }};
        margin: 10.0;
        size: (300.0, 200.0);
        align: Alignment::CENTER;
        font_size: 28;
        => {
            text(t)
        }
    }
}

#[allow(unused)]
macro_rules! temp {
    () => {
        let t = var("Click Me!");

        let is_hovered = var(Default::default());
        let is_pressed = var(Default::default());
        let iv = merge_var!(is_hovered.clone(), is_pressed.clone(), |&hovered, &pressed| {
            if pressed {
                2
            } else if hovered {
                1
            } else {
                0
            }
        });    
        let background_color = switch_var!(iv, ButtonBackground, ButtonBackgroundHovered, ButtonBackgroundPressed);
    
        button! {
            on_click: enclose!{ (t,  background_color) move |a| {
                let ctx = a.ctx();
                ctx.updates.push_set(&t, "Clicked!".into(), ctx.vars).unwrap();
            }};
            is_hovered: is_hovered;
            is_pressed: is_pressed;
            margin: 10.0;
            size: (300.0, 200.0);
            background_color: background_color;
            align: Alignment::CENTER;
            font_size: 28;
            => {
                text(t)
            }
        }
    };
}
