#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // zero_ui_view::run_same_process(app_main);
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            title = "Gradient Example";
            auto_size = true;
            content = scrollable! {
                padding = 20;
                content = v_stack! {
                    spacing = 20;
                    items = widgets![
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

fn title(title: &'static str) -> impl Widget {
    text! {
        text = title;
        font_size = 18.pt();
    }
}

fn linear_angle() -> impl Widget {
    sample_line(widgets![
        sample("90º", linear_gradient(90.deg(), [colors::RED, colors::BLUE])),
        sample("45º", linear_gradient(45.deg(), [colors::GREEN, colors::BLUE])),
        sample("0º", linear_gradient(0.deg(), [colors::BLACK, colors::GREEN])),
        sample("45º 14px", linear_gradient(45.deg(), [(colors::LIME, 14), (colors::GRAY, 14)])),
    ])
}

fn linear_points() -> impl Widget {
    sample_line(widgets![
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
    ])
}

fn linear_tile() -> impl Widget {
    let w = 180 / 5;
    sample_line(widgets![
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
    ])
}

fn stack_linear() -> impl Widget {
    sample_line(widgets![
        sample(
            "background",
            z_stack(nodes![
                linear_gradient(45.deg(), [colors::RED, colors::GREEN]),
                linear_gradient(135.deg(), [rgba(0, 0, 255, 0.5), rgba(1.0, 1.0, 1.0, 0.5)]),
            ]),
        ),
        sample(
            "over color",
            z_stack(nodes![
                fill_color(colors::WHITE),
                linear_gradient(0.deg(), stops![colors::RED, (colors::RED.transparent(), 50.pct())]),
                linear_gradient(120.deg(), stops![colors::GREEN, (colors::GREEN.transparent(), 50.pct())]),
                linear_gradient(240.deg(), stops![colors::BLUE, (colors::BLUE.transparent(), 50.pct())]),
            ]),
        ),
        sample(
            "rainbow",
            z_stack({
                let rainbow = GradientStops::from_stripes(
                    &[
                        colors::RED,
                        colors::ORANGE,
                        colors::YELLOW,
                        colors::GREEN,
                        colors::DODGER_BLUE,
                        colors::INDIGO,
                        colors::BLUE_VIOLET,
                    ],
                    0.0,
                );
                let mut cross_rainbow = rainbow.clone();
                cross_rainbow.set_alpha(0.5);
                nodes![
                    linear_gradient(Line::to_right(), rainbow),
                    linear_gradient(Line::to_bottom(), cross_rainbow),
                ]
            }),
        ),
        sample(
            "angles",
            z_stack({
                fn gradient(angle: i32, mut color: Rgba) -> impl UiNode {
                    color.alpha = 0.3;
                    let stops = GradientStops::from_stripes(&[color, color.transparent()], 0.0);
                    linear_gradient(angle.deg(), stops)
                }

                nodes![
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
                ]
            }),
        ),
    ])
}

fn sample(name: impl ToText, gradient: impl UiNode) -> impl Widget {
    let name = name.to_text();
    v_stack! {
        spacing = 5;
        items = widgets![
            text(name),
            container! {
                size = (180, 180);
                content_align = unset!;
                content = gradient;
            }
        ];
    }
}

fn sample_line(items: impl WidgetList) -> impl Widget {
    h_stack! {
        spacing = 5;
        items;
    }
}
