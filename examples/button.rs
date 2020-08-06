use enclose::enclose;
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Button Example";
            content: v_stack! {
                spacing: 5.0;
                items: ui_vec![example(), example()];
            };
        }
    })
}

fn example() -> impl Widget {
    let t = var("Click Me!");
    let mut count = 0;

    button! {
        on_click: enclose!{ (t) move |a| {
            let ctx = a.ctx();
            count += 1;
            ctx.updates.push_set(&t, formatx!("Clicked {} time{}!", count, if count > 1 {"s"} else {""}), ctx.vars).unwrap();
        }};

        content: text(t);
    }
}
