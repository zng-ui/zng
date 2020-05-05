#![recursion_limit = "256"]

use enclose::enclose;

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
macro_rules! TODO {
    () => {
        button! {
            on_click: |_|println!("Button clicked!");
            // remove `=>`?
            content: {
                container! {
                    content: "Click Me!"
                }
            };

            margin: 10.0;
            size: (300.0, 200.0);
            align: Alignment::CENTER;
            font_size: 28;
    
            // when only at the end?
            when self.is_pressed {
                font_size: 30;
            }
        }
    };
}