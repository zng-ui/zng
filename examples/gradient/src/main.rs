//! Demonstrates gradient rendering.

use zng::{
    color::{
        self,
        gradient::{GradientStops, linear_gradient, stops},
    },
    layout::{Line, size},
    prelude::*,
    text::ToTxt,
};

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        Window! {
            title = "Gradient Example";
            auto_size = true;
            icon = WindowIcon::render(icon);
            child = Scroll! {
                padding = 20;
                child = Stack! {
                    direction = StackDirection::top_to_bottom();
                    spacing = 20;
                    children = ui_vec![
                        title("Linear"),
                        linear_angle(),
                        linear_points(),
                        linear_tile(),
                        title("Stack"),
                        stack_linear(),
                    ];
                };
            };
        }
    });
}

fn title(title: &'static str) -> UiNode {
    Text! {
        txt = title;
        font_size = 18.pt();
    }
}

fn linear_angle() -> UiNode {
    sample_line(ui_vec![
        sample("90º", linear_gradient(90.deg(), [web_colors::RED, web_colors::BLUE])),
        sample("45º", linear_gradient(45.deg(), [web_colors::GREEN, web_colors::BLUE])),
        sample("0º", linear_gradient(0.deg(), [web_colors::BLACK, web_colors::GREEN])),
        sample(
            "45º 14px",
            linear_gradient(45.deg(), [(web_colors::LIME, 14), (web_colors::GRAY, 14)])
        ),
    ])
}

fn linear_points() -> UiNode {
    sample_line(ui_vec![
        sample(
            "(30, 30) to (90, 90) clamp",
            linear_gradient((30, 30).to(90, 90), [web_colors::GREEN, web_colors::RED]),
        ),
        sample(
            "(30, 30) to (90, 90) repeat",
            linear_gradient((30, 30).to(90, 90), [web_colors::GREEN, web_colors::RED]).repeat(),
        ),
        sample(
            "(30, 30) to (90, 90) reflect",
            linear_gradient((30, 30).to(90, 90), [web_colors::GREEN, web_colors::RED]).reflect(),
        ),
        sample(
            "to bottom right",
            linear_gradient(
                Line::to_bottom_right(),
                stops![web_colors::MIDNIGHT_BLUE, 80.pct(), web_colors::CRIMSON]
            ),
        ),
    ])
}

fn linear_tile() -> UiNode {
    let w = 180 / 5;
    sample_line(ui_vec![
        sample(
            "tiles",
            linear_gradient(45.deg(), [web_colors::GREEN, web_colors::YELLOW]).tile(w, 0),
        ),
        sample(
            "tiles spaced",
            linear_gradient(45.deg(), [web_colors::MAGENTA, web_colors::AQUA]).tile(w + 5, 5),
        ),
        sample(
            "pattern",
            linear_gradient(45.deg(), [(web_colors::BLACK, 50.pct()), (web_colors::ORANGE, 50.pct())]).tile(20, 0),
        ),
    ])
}

fn stack_linear() -> UiNode {
    sample_line(ui_vec![
        sample(
            "background",
            ui_vec![
                linear_gradient(45.deg(), [web_colors::RED, web_colors::GREEN]),
                linear_gradient(135.deg(), [rgba(0, 0, 255, 0.5), rgba(1.0, 1.0, 1.0, 0.5)]),
            ],
        ),
        sample(
            "over color",
            ui_vec![
                color::flood(web_colors::WHITE),
                linear_gradient(0.deg(), stops![web_colors::RED, (web_colors::RED.transparent(), 50.pct())]),
                linear_gradient(120.deg(), stops![web_colors::GREEN, (web_colors::GREEN.transparent(), 50.pct())]),
                linear_gradient(240.deg(), stops![web_colors::BLUE, (web_colors::BLUE.transparent(), 50.pct())]),
            ],
        ),
        sample("rainbow", {
            let rainbow = GradientStops::from_stripes(
                &[
                    web_colors::RED,
                    web_colors::ORANGE,
                    web_colors::YELLOW,
                    web_colors::GREEN,
                    web_colors::DODGER_BLUE,
                    web_colors::INDIGO,
                    web_colors::BLUE_VIOLET,
                ],
                0.0,
            );
            let mut cross_rainbow = rainbow.clone();
            cross_rainbow.set_alpha(0.5);
            ui_vec![
                linear_gradient(Line::to_right(), rainbow),
                linear_gradient(Line::to_bottom(), cross_rainbow),
            ]
        },),
        sample("angles", {
            fn gradient(angle: i32, mut color: color::Rgba) -> UiNode {
                color.alpha = 0.3;
                let stops = GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                linear_gradient(angle.deg(), stops).into_node()
            }

            ui_vec![
                color::flood(web_colors::WHITE),
                gradient(0, web_colors::RED),
                gradient(20, web_colors::RED),
                gradient(40, web_colors::RED),
                gradient(120, web_colors::GREEN),
                gradient(140, web_colors::GREEN),
                gradient(160, web_colors::GREEN),
                gradient(240, web_colors::BLUE),
                gradient(260, web_colors::BLUE),
                gradient(280, web_colors::BLUE),
            ]
        },),
    ])
}

fn sample(name: impl ToTxt, gradient: impl IntoUiNode) -> UiNode {
    let name = name.to_txt();
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![
            Text!(name),
            Container! {
                size = 180;
                child = gradient;
            }
        ];
    }
}

fn sample_line(children: impl IntoUiNode) -> UiNode {
    Stack! {
        direction = StackDirection::left_to_right();
        spacing = 5;
        children;
    }
}

fn icon() -> UiNode {
    Text! {
        size = 36;
        widget::background_gradient = Line::to_bottom_right(), stops![web_colors::MIDNIGHT_BLUE, 70.pct(), web_colors::CRIMSON];
        widget::corner_radius = 6;
        font_size = 28;
        font_weight = FontWeight::BOLD;
        txt_align = Align::CENTER;
        txt = "G";
    }
}
