#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::{units::AngleRadian, *};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("transform");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        Window! {
            title = "Transform Example";
            child = Stack! {
                children = ui_vec![
                    Stack! {
                        align = Align::CENTER;
                        direction = StackDirection::left_to_right();
                        spacing = 40;
                        children = ui_vec![
                            transform2d_examples(),
                            transform3d_examples(),
                            Stack! {
                                direction = StackDirection::top_to_bottom();
                                children_align = Align::TOP_LEFT;
                                spacing = 40;
                                children = ui_vec![
                                    transform_stack(),
                                    transform_order(),
                                    cube_example(),
                                ]
                            }
                        ]
                    },
                    open_touch_example()
                ]
            };
        }
    })
}

fn transform2d_examples() -> impl UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 25;
        children_align = Align::TOP;
        children = ui_vec![
            transformed("Translate -10", translate(-10, -10)),
            transformed_at("Rotate 10º (0, 0)", rotate(10.deg()), (0, 0)),
            transformed("Rotate 10º", rotate(10.deg())),
            transformed("Skew-X 15º", skew_x(15.deg())),
            transformed("Scale 130%", scale(130.pct())),
            transformed("Identity", Transform::identity()),
            transformed_sampler("Lerp", animation::Transition::sample),
            transformed_sampler("Slerp", units::slerp_sampler),
        ];
    }
}

fn transform3d_examples() -> impl UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 25;
        children_align = Align::TOP;
        children = ui_vec![
            transformed_3d("Rotate Y:45º (.5, .5)", rotate_y(45.deg()), Point::center()),
            transformed_3d("Rotate Y:45º (0., 0.)", rotate_y(45.deg()), Point::top_left()),
            transformed_3d("Rotate Y:45º (1., 1.)", rotate_y(45.deg()), Point::bottom_right()),
            transformed_3d("Rotate Y:145º (.5, .5)", rotate_y(145.deg()), Point::center()),
            transformed_3d("Translate Z 50", translate_z(50), Point::center()),
            Container! {
                child = Container! {
                    transform = rotate_y(45.deg()).translate_z(20);
                    child = Text!("Perspective");
                    background_color = color_scheme_map(colors::BLUE.with_alpha(80.pct()), hex!(#62BEFC).with_alpha(80.pct()));
                    padding = 10;
                };

                background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
                transform_style = TransformStyle::Preserve3D;
                border = 2, (colors::GRAY, BorderStyle::Dashed);

                #[easing(300.ms())]
                perspective = 700;
                when *#is_hovered {
                    perspective = 100;
                }
            },
            Container! {
                perspective = 600;
                child = {
                    let show_front = var(true);
                    Container! {
                        tooltip = Tip!(Text!("Click to flip"));

                        transform_style = TransformStyle::Preserve3D;
                        #[easing(300.ms())]
                        rotate_y = show_front.map(|&f| if f { 0.deg() } else { 180.deg() }.into());
                        on_click = hn!(|_| {
                            show_front.set(!show_front.get());
                        });
                        size = (100, 80);
                        corner_radius = 5;
                        backface_visibility = false;

                        child = Text! {
                            background_color = colors::GREEN.with_alpha(70.pct());
                            txt_align = Align::CENTER;
                            font_weight = FontWeight::BOLD;
                            font_size = 24;
                            txt = "FRONT";
                        };
                        background = Text! {
                            rotate_y = 180.deg();
                            background_color = colors::BLUE.lighten(50.pct()).with_alpha(70.pct());
                            txt_align = Align::CENTER;
                            font_weight = FontWeight::BOLD;
                            font_size = 24;
                            txt = "BACK";
                        };
                    }
                }
            }
        ];
    }
}

fn transformed(label: impl Into<Txt>, transform: Transform) -> impl UiNode {
    let parent_is_hovered = var(false);
    Container! {
        child = Container! {
            #[easing(300.ms())]
            transform;
            child = Text!(label.into());
            background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
            padding = 10;

            when *#is_hovered || *#{parent_is_hovered.clone()} {
                transform = Transform::identity();
            }
        };
        is_hovered = parent_is_hovered;
        border = 2, (colors::GRAY, BorderStyle::Dashed);
    }
}
fn transformed_3d(label: impl Into<Txt>, transform: Transform, origin: Point) -> impl UiNode {
    let parent_is_hovered = var(false);
    Container! {
        child = Container! {
            #[easing(300.ms())]
            transform;
            child = Text!(label.into());
            background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
            padding = 10;

            when *#is_hovered || *#{parent_is_hovered.clone()} {
                transform = Transform::identity();
            }
        };

        is_hovered = parent_is_hovered;
        perspective = 400;
        perspective_origin = origin;
        transform_style = TransformStyle::Preserve3D;
        border = 2, (colors::GRAY, BorderStyle::Dashed);
    }
}
fn transformed_at(label: impl Into<Txt>, transform: Transform, origin: impl Into<Point>) -> impl UiNode {
    let parent_is_hovered = var(false);
    Container! {
        child = Container! {
            #[easing(300.ms())]
            transform;
            transform_origin = origin.into();

            child = Text!(label.into());
            background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
            padding = 10;

            when *#is_hovered || *#{parent_is_hovered.clone()} {
                transform = Transform::identity();
            }
        };
        is_hovered = parent_is_hovered;
        border = 2, (colors::GRAY, BorderStyle::Dashed);
    }
}
fn transformed_sampler(
    label: impl Into<Txt>,
    sampler: impl Fn(&animation::Transition<AngleRadian>, EasingStep) -> AngleRadian + Send + Sync + 'static,
) -> impl UiNode {
    let parent_is_hovered = var(false);
    Container! {
        child = {
            let is_hovered = var(false);
            Container! {
                rotate = merge_var!(is_hovered.clone(), parent_is_hovered.clone(), |h, ph| *h || *ph)
                    .map(|&hovered| if !hovered { 20.deg() } else { (360 - 20).deg() }.into())
                    .easing_with(300.ms(), easing::linear, sampler);

                child = Text!(label.into());
                background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
                padding = 10;

                is_hovered;
            }
        };
        is_hovered = parent_is_hovered;
        border = 2, (colors::GRAY, BorderStyle::Dashed);
    }
}

fn transform_stack() -> impl UiNode {
    // the panel widget uses its child transform to position the widget for performance reasons,
    // the widget transform does not affect.
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![
            Container! {
                child = Text!("Identity");
                background_color = colors::DARK_GRAY.with_alpha(80.pct());
                padding = 10;
            },
            Container! {
                id = "in-stack";
                transform = rotate(45.deg());
                child = Text!("Rotated 45º");
                background_color = color_scheme_map(colors::BROWN.with_alpha(80.pct()), hex!(#EF6950).with_alpha(80.pct()));
                padding = 10;

                when *#is_hovered {
                    z_index = ZIndex::DEFAULT + 1;
                }
            },
            Container! {
                child = Text!("Identity");
                background_color = colors::DARK_GRAY.with_alpha(80.pct());
                padding = 10;
            },
        ];
    }
}

fn transform_order() -> impl UiNode {
    // transform created using a single property or two properties generate the same transform because
    // are in the same order.
    z_stack(ui_vec![
        Wgt! {
            // single property
            transform = rotate(10.deg()).translate(-5, -5);

            size = (60, 60);
            background_color = colors::BLUE.lighten(50.pct());

            when *#is_hovered {
                z_index = ZIndex::DEFAULT + 1;
            }
        },
        Wgt! {
            // two properties
            rotate = 10.deg();
            translate = -5, -5;

            size = (60, 60);
            background_color = colors::GREEN;

            when *#is_hovered {
                z_index = ZIndex::DEFAULT - 1;
            }
        },
    ])
}

#[allow(clippy::precedence)]
fn cube_example() -> impl UiNode {
    // Based on https://codepen.io/desandro/pen/KRWjzm?editors=1100
    let show = var(1u8);
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![
            Container! {
                id = "scene";
                size = 200;
                perspective = 400;

                child = Stack! {
                    id = "cube";
                    transform_style = TransformStyle::Preserve3D;

                    children = (1..=6u8).map(|i| Text! {
                        txt = i.to_text();
                        // size = 200;
                        font_size = 62;
                        font_weight = FontWeight::BOLD;
                        txt_align = Align::CENTER;
                        background_color = hsla((360.0 * (7.0 / i as f32)).deg(), 0.5, 0.5, 0.7);
                        border = 2, text::FONT_COLOR_VAR.map_into();

                        transform = translate_z(100).then(match i {
                            1 => rotate_y(0.deg()),
                            2 => rotate_y(90.deg()),
                            3 => rotate_y(180.deg()),
                            4 => rotate_y(-90.deg()),
                            5 => rotate_x(90.deg()),
                            6 => rotate_x(-90.deg()),
                            _ => unreachable!()
                        });
                    }.boxed())
                    .collect::<UiNodeVec>();

                    transform = show.map(|&i| match i {
                        1 => rotate_y(0.deg()),
                        2 => rotate_y(-90.deg()),
                        3 => rotate_y(-180.deg()),
                        4 => rotate_y(90.deg()),
                        5 => rotate_x(-90.deg()),
                        6 => rotate_x(90.deg()),
                        _ => unreachable!(),
                    }
                    .translate_z(-100))
                    .easing_with(1.secs(), easing::linear, units::slerp_sampler)
                }
            },
            Wrap! {
                align = Align::CENTER;
                toggle::selector = toggle::Selector::single(show.clone());
                spacing = 5;
                children = (1..=6u8).map(|i| Toggle! {
                    style_fn = toggle::RadioStyle!();
                    value::<u8> = i;
                    child = Text!(i.to_text());
                }).collect::<UiNodeVec>();
            }
        ];
    }
}

fn open_touch_example() -> impl UiNode {
    let is_open = var(false);
    Button! {
        align = Align::TOP_END;
        margin = 10;
        style_fn = button::LinkStyle!();
        font_size = 1.1.em();
        child = Text!("Touch Transform");
        on_click = hn!(|_| {
            let example_id = WindowId::named("touch-example");
            if is_open.get() {
                if WINDOWS.focus(example_id).is_err() {
                    is_open.set(false);
                }
            } else {
                let parent = WINDOW.id();
                is_open.set(true);
                WINDOWS.open_id(example_id, async_clmv!(is_open, {
                    Window! {
                        title = "Transform Example - Touch";
                        state = WindowState::Maximized;
                        parent;
                        child = touch_example();
                        on_close = hn_once!(|_| is_open.set(false));
                    }
                }));
            }
        })
    }
}

fn touch_example() -> impl UiNode {
    use zero_ui::core::touch::TouchTransformMode;

    let mode = var(TouchTransformMode::ALL);

    macro_rules! ModeToggle {
        ($FLAG:ident) => {
            Toggle! {
                style_fn = toggle::CheckStyle!();
                value = TouchTransformMode::$FLAG;
                child = Text!(stringify!($FLAG));
            }
        };
    }

    Stack! {
        children = ui_vec![
            Wgt! {
                capture_pointer = true;
                touch_transform = mode.clone();

                offset = 200;
                size = 400;
                background_color = colors::DODGER_BLUE;
                corner_radius = 10;
            },

            Stack! {
                toggle::selector = toggle::Selector::bitflags(mode);
                direction = StackDirection::top_to_bottom();
                margin = 10;
                spacing = 10;
                font_size = 1.1.em();
                align = Align::TOP_START;
                children = ui_vec![
                    ModeToggle!(TRANSLATE_X),
                    ModeToggle!(TRANSLATE_Y),
                    ModeToggle!(TRANSLATE),
                    ModeToggle!(SCALE_X),
                    ModeToggle!(SCALE_Y),
                    ModeToggle!(SCALE),
                    ModeToggle!(ROTATE),
                    ModeToggle!(ALL),
                ]
            }
        ]
    }
}
