use zero_ui::prelude::*;

fn main() {
    better_panic::install();

    App::default().run_window(|_| {
        let size = var((800., 600.));
        let title = size.map(|s: &LayoutSize| formatx!("Window Example - {}x{}", s.width.ceil(), s.height.ceil()));
        let background_color = var(rgb(0.1, 0.1, 0.1));
        window! {
            size: size.clone();
            background_color: background_color.clone();
            title;
            content: h_stack! {
                spacing: 40.0;
                items: ui_vec![
                    property_stack("size", ui_vec![
                        set_size(1000.0, 900.0, &size),
                        set_size(500.0, 1000.0, &size),
                        set_size(800.0, 600.0, &size),
                    ]),
                    property_stack("background_color", ui_vec![
                        set_background(rgb(0.1, 0.1, 0.1), "default", &background_color),
                        set_background(rgb(0.5, 0.0, 0.0), "red", &background_color),
                        set_background(rgb(0.0, 0.5, 0.0), "green", &background_color),
                        set_background(rgb(0.0, 0.0, 0.5), "blue", &background_color),
                    ])
                ];
            };
        }
    })
}

fn property_stack(header: &'static str, mut items: UiVec) -> impl Widget {
    items.insert(
        0,
        text! {
            text: header;
            font_weight: FontWeight::BOLD;
            margin: (0.0, 4.0);
        }
        .boxed(),
    );
    v_stack! {
        spacing: 5.0;
        items;
    }
}

fn set_size(width: f32, height: f32, window_size: &SharedVar<LayoutSize>) -> impl Widget {
    set_var_btn(
        window_size,
        LayoutSize::new(width, height),
        formatx!("resize to {}x{}", width, height),
    )
}

fn set_background(color: ColorF, color_name: &str, background_color: &SharedVar<ColorF>) -> impl Widget {
    set_var_btn(background_color, color, formatx!("{} background", color_name))
}

fn set_var_btn<T: zero_ui::core::var::VarValue>(var: &SharedVar<T>, new_value: T, content_txt: Text) -> impl Widget {
    let var = var.clone();
    button! {
        content: text(content_txt);
        on_click: move |a| {
            let ctx = a.ctx();
            ctx.updates.push_set(&var, new_value.clone(), ctx.vars).unwrap();
        };
    }
}
