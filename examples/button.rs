#![recursion_limit = "256"]

use enclose::enclose;

use zero_ui::prelude::*;

fn main() {
    better_panic::install();

    App::default().run_window(|_| {
        let size = var((800., 600.));
        let title = size.map(|s: &LayoutSize| formatx!("Button Example - {}x{}", s.width.ceil(), s.height.ceil()));
        window! {
            size: size;
            title: title;
            content: v_stack((example(), example()).into());
        }
    })
}

fn example() -> impl UiNode {
    let t = var("Click Me!");

    button! {
        on_click: enclose!{ (t) move |a| {
            let ctx = a.ctx();
            ctx.updates.push_set(&t, "Clicked!".into(), ctx.vars).unwrap();
        }};
        align: Alignment::CENTER;

        content: text_run(t);
    }
}
