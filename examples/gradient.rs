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
                   linear_angle(),
                   linear_points(),
                   linear_tile(),
                );
            };
        }
    })
}

fn linear_angle() -> impl Widget {
    h_stack! {
        spacing: 5;
        items: (
            sample("linear 90º", linear_gradient(90.deg(), [colors::RED, colors::BLUE])),
            sample("linear 45º", linear_gradient(45.deg(), [colors::GREEN, colors::BLUE])),
            sample("linear 0º", linear_gradient(0.deg(), [colors::BLACK, colors::GREEN])),
            sample("clamp", linear_gradient(135.deg(), [(colors::DARK_RED, 49.pct()), (colors::ORANGE, 51.pct())])),
        );
    }
}

fn linear_points() -> impl Widget {
    h_stack! {
        spacing: 5;
        items: (
            sample(
                "linear points - clamp",
                linear_gradient_pt((30, 30), (90, 90), [colors::GREEN, colors::RED], ExtendMode::Clamp)
            ),
            sample(
                "linear points - repeat",
                linear_gradient_pt((30, 30), (90, 90), [colors::GREEN, colors::RED], ExtendMode::Repeat)
            ),
            sample(
                "test",
                linear_gradient_pt((90, 180), (90, 0), [colors::BLACK, colors::GREEN], ExtendMode::Repeat)
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
                "linear tiles",
                linear_gradient_tile(45.deg(), [colors::GREEN, colors::YELLOW], (w, w), (0, 0))
            ),
            sample(
                "linear tiles spaced",
                linear_gradient_tile(45.deg(), [colors::MAGENTA, colors::AQUA], (w + 5, w + 5), (5, 5))

            ),
        );
    }
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
