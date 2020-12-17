use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Gradient Example";
            auto_size: true;
            padding: 20;
            content: v_stack! {
                spacing: 20;
                items: (
                    title("Linear"),
                    linear_angle(),
                    linear_points(),
                    linear_tile(),
                    title("Stack"),
                    stack_linear(),
                );
            };
        }
    })
}

fn title(title: &'static str) -> impl Widget {
    text! {
        text: title;
        font_size: 18.pt();
    }
}

fn linear_angle() -> impl Widget {
    h_stack! {
        spacing: 5;
        items: (
            sample("90º", linear_gradient(90.deg(), [colors::RED, colors::BLUE], ExtendMode::Clamp)),
            sample("45º", linear_gradient(45.deg(), [colors::GREEN, colors::BLUE], ExtendMode::Clamp)),
            sample("0º", linear_gradient(0.deg(), [colors::BLACK, colors::GREEN], ExtendMode::Clamp)),
            sample("45º 14px", linear_gradient(45.deg(), [(colors::LIME, 14), (colors::GRAY, 14)], ExtendMode::Clamp)),
        );
    }
}

fn linear_points() -> impl Widget {
    h_stack! {
        spacing: 5;
        items: (
            sample(
                "(30, 30) to (90, 90) clamp",
                linear_gradient_pt((30, 30), (90, 90), [colors::GREEN, colors::RED], ExtendMode::Clamp)
            ),
            sample(
                "(30, 30) to (90, 90) repeat",
                linear_gradient_pt((30, 30), (90, 90), [colors::GREEN, colors::RED], ExtendMode::Repeat)
            ),
            sample(
                "to bottom right",
                linear_gradient_to_bottom_right(stops![colors::MIDNIGHT_BLUE, 80.pct(), colors::CRIMSON], ExtendMode::Clamp)
            ),
        );
    }
}

fn linear_tile() -> impl Widget {
    let w = 180 / 5;
    h_stack! {
        spacing: 5;
        items: (
            sample(
                "tiles",
                linear_gradient_tile(45.deg(), [colors::GREEN, colors::YELLOW], ExtendMode::Clamp, (w, w), (0, 0))
            ),
            sample(
                "tiles spaced",
                linear_gradient_tile(45.deg(), [colors::MAGENTA, colors::AQUA], ExtendMode::Clamp, (w + 5, w + 5), (5, 5))
            ),
            sample(
                "pattern",
                linear_gradient_tile(45.deg(), [(colors::BLACK, 50.pct()), (colors::ORANGE, 50.pct())], ExtendMode::Clamp, (20, 20), (0, 0))
            ),
        );
    }
}

// TODO
fn stack_linear() -> impl Widget {
    sample_line((
        sample("stack 2", z_stack((
            linear_gradient(45.deg(), [colors::RED, colors::GREEN], ExtendMode::Clamp),
            linear_gradient(135.deg(), [rgba(0, 0, 255, 0.5), rgba(1.0, 1.0, 1.0, 0.5)], ExtendMode::Clamp),
        ))),
        sample("stack 3", z_stack((
            fill_color(colors::WHITE),
            linear_gradient_to_bottom_left(stops![rgba(255, 0, 0, 1.0), (rgba(255, 0, 0, 0.0), 50.pct())], ExtendMode::Clamp),      
            linear_gradient_to_right(stops![rgba(0, 255, 0, 1.0), (rgba(0, 255, 0, 0.0), 50.pct())], ExtendMode::Clamp),            
            linear_gradient_to_top_left( stops![rgba(0, 0, 255, 1.0), (rgba(0, 0, 255, 0.0), 50.pct())], ExtendMode::Clamp),
        ))),
        sample("rainbow", z_stack(
            {
                let stops = stops![colors::RED, (colors::YELLOW, 0.333.normal()), (colors::GREEN, 0.5.normal()), (colors::CYAN, 0.666.normal()), (colors::BLUE, 0.833.normal()), colors::MAGENTA];
                let mut stops2 = stops.clone();
                stops2.start.color.alpha = 0.75;
                for stop in &mut stops2.middle {
                    if let GradientStop::Color(color_stop) = stop {
                        color_stop.color.alpha = 0.75;
                    }
                }
                stops2.end.color.alpha = 0.75;

                (linear_gradient_to_right(stops, ExtendMode::Clamp), linear_gradient_to_bottom(stops2, ExtendMode::Clamp))
            }
        ))
    ))
    
}

fn sample(name: impl ToText, gradient: impl UiNode) -> impl Widget {
    let name = name.to_text();
    v_stack! {
        spacing: 5;
        items: (
            text(name),
            container! {
                background: gradient;
                size: (180, 180);
                content: text("");
            }
        );
    }
}

fn sample_line(items: impl WidgetList) -> impl Widget {
    h_stack! {
        spacing: 5;
        items;
    }
}
