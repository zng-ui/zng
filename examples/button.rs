#![recursion_limit = "256"]

#[macro_use]
extern crate zero_ui;
use zero_ui::prelude::*;

fn main() {
    App::default().run(|ctx| {
        //ctx.services.req::<Windows>().open(|ctx| {
        //    window! {
        //        title: "Button Example";
        //        => example()
        //    }
        //})
    })
}

fn example() -> impl UiNode {
    let content = var("Click Me!".to_text());
    button! {
        on_click: { let c = content.clone(); move |a| {
            a.ctx().updates.push_set(&c, "Clicked!".to_text());
        }};
        align: Alignment::CENTER;
        font_size: 28;
        text_color: rgb(0, 100, 200);
        => text(content)
    }
}
