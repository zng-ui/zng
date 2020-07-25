use zero_ui::prelude::*;

fn main() {
    better_panic::install();

    App::default().run_window(|_| {
        let size = var((800., 600.));
        let title = size.map(|s: &LayoutSize| formatx!("Window Example - {}x{}", s.width.ceil(), s.height.ceil()));
        window! {
            size: size.clone();
            title;
            content: v_stack! {
                spacing: 5.0;
                items: ui_vec![
                    set_size(1000.0, 900.0, &size),
                    set_size(500.0, 1000.0, &size),
                    set_size(800.0, 600.0, &size),
                ];
            };
        }
    })
}

fn set_size(width: f32, height: f32, window_size: &SharedVar<LayoutSize>) -> impl Widget {
    let window_size = window_size.clone();
    button! {
        content: text(formatx!("resize to {}x{}", width, height));
        on_click: move |a| {
            let ctx = a.ctx();
            ctx.updates.push_set(&window_size, LayoutSize::new(width, height), ctx.vars).unwrap();
        };
    }
}
