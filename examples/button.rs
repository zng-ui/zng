#![recursion_limit = "256"]

#[macro_use]
extern crate zero_ui;
#[macro_use]
extern crate enclose;

use zero_ui::prelude::*;

fn main() {
    App::default().run(|ctx| {
        //ctx.services.req::<Windows>().open(|ctx| {
        //    window! {
        //        title: "Button Example";
        //        => example()
        //    }
        //});
    })
}

fn example() -> impl UiNode {
    let t = var("Click Me!");
    button! {
        on_click: enclose!{ (t) move |a| {
            a.ctx().updates.push_set(&t, "Clicked!".to_text()).ok();
        }};
        align: Alignment::CENTER;
        font_size: 28;
        text_color: rgb(0, 100, 200);
        => {
            text(t)
        }
    }
}
