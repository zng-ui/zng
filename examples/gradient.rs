use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            title = "Gradient Example";
            auto_size = true;
            padding = 20;
            content = v_stack! {
                spacing = 20;
                items = (
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
        text = title;
        font_size = 18.pt();
    }
}

fn linear_angle() -> impl Widget {
    sample_line((
        sample("90º", linear_gradient(90.deg(), [colors::RED, colors::BLUE])),
        sample("45º", linear_gradient(45.deg(), [colors::GREEN, colors::BLUE])),
        sample("0º", linear_gradient(0.deg(), [colors::BLACK, colors::GREEN])),
        sample("45º 14px", linear_gradient(45.deg(), [(colors::LIME, 14), (colors::GRAY, 14)])),
    ))
}

fn linear_points() -> impl Widget {
    sample_line((
        sample(
            "(30, 30) to (90, 90) clamp",
            linear_gradient((30, 30).to(90, 90), [colors::GREEN, colors::RED]),
        ),
        sample(
            "(30, 30) to (90, 90) repeat",
            repeating_linear_gradient((30, 30).to(90, 90), [colors::GREEN, colors::RED]),
        ),
        sample(
            "(30, 30) to (90, 90) reflect",
            reflecting_linear_gradient((30, 30).to(90, 90), [colors::GREEN, colors::RED]),
        ),
        sample(
            "to bottom right",
            linear_gradient(Line::to_bottom_right(), stops![colors::MIDNIGHT_BLUE, 80.pct(), colors::CRIMSON]),
        ),
    ))
}

fn linear_tile() -> impl Widget {
    let w = 180 / 5;
    sample_line((
        sample(
            "tiles",
            linear_gradient_full(45.deg(), [colors::GREEN, colors::YELLOW], ExtendMode::Clamp, (w, w), (0, 0)),
        ),
        sample(
            "tiles spaced",
            linear_gradient_full(45.deg(), [colors::MAGENTA, colors::AQUA], ExtendMode::Clamp, (w + 5, w + 5), (5, 5)),
        ),
        sample(
            "pattern",
            linear_gradient_full(
                45.deg(),
                [(colors::BLACK, 50.pct()), (colors::ORANGE, 50.pct())],
                ExtendMode::Clamp,
                (20, 20),
                (0, 0),
            ),
        ),
    ))
}

fn stack_linear() -> impl Widget {
    sample_line((
        sample(
            "background",
            z_stack((
                linear_gradient(45.deg(), [colors::RED, colors::GREEN]),
                linear_gradient(135.deg(), [rgba(0, 0, 255, 0.5), rgba(1.0, 1.0, 1.0, 0.5)]),
            )),
        ),
        sample(
            "over color",
            z_stack((
                fill_color(colors::WHITE),
                linear_gradient(0.deg(), stops![colors::RED, (colors::RED.transparent(), 50.pct())]),
                linear_gradient(120.deg(), stops![colors::GREEN, (colors::GREEN.transparent(), 50.pct())]),
                linear_gradient(240.deg(), stops![colors::BLUE, (colors::BLUE.transparent(), 50.pct())]),
            )),
        ),
        sample(
            "rainbow",
            z_stack({
                let rainbow = GradientStops::from_stripes(&[
                    colors::RED,
                    colors::ORANGE,
                    colors::YELLOW,
                    colors::GREEN,
                    colors::DODGER_BLUE,
                    colors::INDIGO,
                    colors::BLUE_VIOLET,
                ]);
                let mut cross_rainbow = rainbow.clone();
                cross_rainbow.start.color.alpha = 0.5;
                for stop in &mut cross_rainbow.middle {
                    if let GradientStop::Color(color_stop) = stop {
                        color_stop.color.alpha = 0.5;
                    }
                }
                cross_rainbow.end.color.alpha = 0.5;

                (
                    linear_gradient(Line::to_right(), rainbow),
                    linear_gradient(Line::to_bottom(), cross_rainbow),
                )
            }),
        ),
        sample(
            "angles",
            z_stack({
                fn gradient(angle: i32, mut color: Rgba) -> impl UiNode {
                    color.alpha = 0.3;
                    let stops = GradientStops::from_stripes(&[color, color.transparent()]);
                    linear_gradient(angle.deg(), stops)
                }

                (
                    fill_color(colors::WHITE),
                    gradient(0, colors::RED),
                    gradient(20, colors::RED),
                    gradient(40, colors::RED),
                    gradient(120, colors::GREEN),
                    gradient(140, colors::GREEN),
                    gradient(160, colors::GREEN),
                    gradient(240, colors::BLUE),
                    gradient(260, colors::BLUE),
                    gradient(280, colors::BLUE),
                )
            }),
        ),
    ))
}

fn sample(name: impl ToText, gradient: impl UiNode) -> impl Widget {
    let name = name.to_text();
    v_stack! {
        spacing = 5;
        items = (
            text(name),
            container! {
                background = gradient;
                size = (180, 180);
                content = text("");
            }
        );
    }
}

fn sample_line(items: impl WidgetList) -> impl Widget {
    h_stack! {
        spacing = 5;
        items;
    }
}
