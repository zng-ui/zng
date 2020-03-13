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
            let title = var("Button Example");
            window! {
                background_color: rgb(0.1, 0.1, 0.1);
                title: title.clone();
                => example(title)
            }
        });
    })
}

fn example(title: impl Var<Text>) -> impl UiNode {
    let t = var("Click Me!");
    button! {
        on_click: enclose!{ (t) move |a| {
            println!("handler click");
            let u = &mut a.ctx().updates;
            u.push_set(&title, "Clicked!".to_text()).ok();
            u.push_set(&t, "Clicked!".to_text()).ok();
        }};
        content_align: Alignment::BOTTOM_RIGHT;
        size: (300.0, 200.0);
        align: Alignment::CENTER;
        font_size: 28;
        text_color: rgb(0, 100, 200);
        margin: 10.0;
        => {
            text(t)
        }
    }
}
