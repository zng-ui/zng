#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let position = var_from((f32::NAN, f32::NAN));
        let size = var_from((800, 600));

        let title = merge_var!(position.clone(), size.clone(), |p: &Point, s: &Size| {
            formatx!("Window Example - position: {:.0}, size: {:.0}", p, s)
        });
        let background_color = var(rgb(0.1, 0.1, 0.1));

        window! {
            position = position.clone();
            size = size.clone();
            background_color = background_color.clone();
            title;
            content = h_stack! {
                spacing = 40;
                items = (
                    v_stack! {
                        spacing = 20;
                        items = (
                            property_stack("position", [
                                set_position(0.0, 0.0, &position),
                                set_position(490.0, 290.0, &position),
                                set_position(500.0, 300.0, &position),
                            ]),
                            property_stack("miscellaneous", (
                                screenshot(),
                                inspect(),
                            )),
                        );
                    },
                    property_stack("size", [
                        set_size(1000.0, 900.0, &size),
                        set_size(500.0, 1000.0, &size),
                        set_size(800.0, 600.0, &size),
                    ]),
                    property_stack("background_color", [
                        set_background(rgb(0.1, 0.1, 0.1), "default", &background_color),
                        set_background(rgb(0.5, 0.0, 0.0), "red", &background_color),
                        set_background(rgb(0.0, 0.5, 0.0), "green", &background_color),
                        set_background(rgb(0.0, 0.0, 0.5), "blue", &background_color),
                    ])
                );
            };
        }
    })
}

fn property_stack(header: &'static str, items: impl WidgetList) -> impl Widget {
    v_stack! {
        spacing = 5;
        items = [text! {
            text = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}

fn set_position(x: f32, y: f32, window_position: &RcVar<Point>) -> impl Widget {
    set_var_btn(window_position, (x, y).into(), formatx!("move to {}x{}", x, y))
}

fn set_size(width: f32, height: f32, window_size: &RcVar<Size>) -> impl Widget {
    set_var_btn(window_size, (width, height).into(), formatx!("resize to {}x{}", width, height))
}

fn set_background(color: Rgba, color_name: &str, background_color: &RcVar<Rgba>) -> impl Widget {
    set_var_btn(background_color, color, formatx!("{} background", color_name))
}

fn set_var_btn<T: zero_ui::core::var::VarValue>(var: &RcVar<T>, new_value: T, content_txt: Text) -> impl Widget {
    let var = var.clone();
    button! {
        content = text(content_txt);
        on_click = move |ctx, _| {
            var.set(ctx.vars,  new_value.clone());
        };
    }
}

fn screenshot() -> impl Widget {
    use std::time::Instant;
    button! {
        content = text("screenshot");
        on_click = |ctx, _| {
            println!("taking `screenshot.png`..");

            let t = Instant::now();
            let img = ctx.services.req::<Windows>().window(ctx.path.window_id()).unwrap().screenshot();
            println!("taken in {:?}", t.elapsed());

            let t = Instant::now();
            img.save("screenshot.png").unwrap();
            println!("saved in {:?}", t.elapsed());
        };
    }
}

fn inspect() -> impl Widget {
    button! {
        content = text("inspector");
        on_click = |_,_| {
            println!("in debug only, press CTRL+SHIFT+I")
        };
    }
}
