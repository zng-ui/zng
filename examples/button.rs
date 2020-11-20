#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use enclose::enclose;
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Button Example";
            content: v_stack! {
                spacing: 5;
                items: (
                    example(),
                    example(),
                    disabled()
                );
            };
        }
    })
}

fn example() -> impl Widget {
    let t = var_from("Click Me!");
    let mut count = 0;

    button! {
        on_click: enclose!{ (t) move |ctx, _| {
            count += 1;
            let new_txt = formatx!("Clicked {} time{}!", count, if count > 1 {"s"} else {""});
            t.set(ctx.vars, new_txt);
        }};
        on_double_click: |_, _| println!("double click!");
        on_triple_click: |_, _| println!("triple click!");
        content: text(t);
    }
}

fn disabled() -> impl Widget {
    button! {
        on_click: |_, _| panic!("disabled button");
        enabled: false;
        content: text("Disabled");
    }
}
