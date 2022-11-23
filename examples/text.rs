#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::text::{Fonts, UnderlinePosition, UnderlineSkip};
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("text");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        let fs = var(Length::Pt(11.0));

        window! {
            title = fs.map(|s| formatx!("Text Example - font_size: {s}"));
            font_size = fs.easing(150.ms(), easing::linear);
            child = z_stack(ui_list![
                h_stack! {
                    align = Align::CENTER;
                    spacing = 40;
                    children = ui_list![
                        v_stack! {
                            spacing = 20;
                            children = ui_list![
                                basic(),
                                defaults(ctx),
                            ];
                        },
                        v_stack! {
                            spacing = 20;
                            children = ui_list![
                                line_height(),
                                line_spacing(),
                                word_spacing(),
                                letter_spacing(),
                            ];
                        },
                        v_stack! {
                            spacing = 20;
                            children = ui_list![
                                decoration_lines(),
                            ]
                        }
                    ];
                },
                container! {
                    align = Align::TOP;
                    margin = 10;
                    child = font_size(fs);
                },
            ])
        }
    })
}

fn font_size(font_size: ArcVar<Length>) -> impl UiNode {
    fn change_size(font_size: &ArcVar<Length>, change: f32, ctx: &mut WidgetContext) {
        font_size.modify(ctx, move |s| {
            *s.to_mut() += Length::Pt(change);
        });
    }
    h_stack! {
        button::vis::extend_style = style_generator!(|_, _| style! {
            padding = (0, 5);
        });
        spacing = 5;
        corner_radius = 4;
        background_color = color_scheme_map(rgba(0, 0, 0, 40.pct()), rgba(1., 1., 1., 40.pct()));
        padding = 4;
        children = ui_list![
            button! {
                child = text("-");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!(Minus), shortcut!(NumpadSubtract)];
                on_click = hn!(font_size, |ctx, _| {
                    change_size(&font_size, -1.0, ctx)
                });
            },
            text! {
                txt = font_size.map(|s| formatx!("{s}"));
            },
            button! {
                child = text("+");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!(Plus), shortcut!(NumpadAdd)];
                on_click = hn!(font_size, |ctx, _| {
                    change_size(&font_size, 1.0, ctx)
                });
            },
        ]
    }
}

fn basic() -> impl UiNode {
    section(
        "basic",
        ui_list![
            text("Basic Text"),
            strong("Strong Text"),
            em("Emphasis Text"),
            text! {
                txt_color = color_scheme_map(colors::LIGHT_GREEN, colors::DARK_GREEN);
                txt = "Colored Text";

                when *#is_hovered {
                    txt_color = color_scheme_map(colors::YELLOW, colors::BROWN);
                }
            },
        ],
    )
}

fn line_height() -> impl UiNode {
    section(
        "line_height",
        ui_list![
            text! {
                txt = "Default: 'Émp Giga Ç'";
                background_color = colors::LIGHT_BLUE;
                txt_color = colors::BLACK;
            },
            text! {
                txt = "150%: 'Émp Giga Ç'";
                background_color = colors::LIGHT_BLUE;
                txt_color = colors::BLACK;
                line_height = 150.pct();
            },
        ],
    )
}

fn line_spacing() -> impl UiNode {
    section(
        "line_spacing",
        ui_list![container! {
            child = text! {
                txt = "Hello line 1!\nHello line 2!\nHover to change `line_spacing`";
                background_color = rgba(0.5, 0.5, 0.5, 0.3);

                when *#is_hovered {
                    line_spacing = 30.pct();
                }
            };
            child_align = Align::TOP;
            min_height = 1.7.em() * 3.fct();
        }],
    )
}

fn word_spacing() -> impl UiNode {
    section(
        "word_spacing",
        ui_list![text! {
            txt = "Word spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                word_spacing = 100.pct();
            }
        }],
    )
}

fn letter_spacing() -> impl UiNode {
    section(
        "letter_spacing",
        ui_list![text! {
            txt = "Letter spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                letter_spacing = 30.pct();
            }
        }],
    )
}

fn decoration_lines() -> impl UiNode {
    section(
        "Decorations",
        ui_list![
            text! {
                txt = "Overline, 1, Dotted,\ndefault color";
                overline = 1, LineStyle::Dotted;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                txt = "Strikethrough, 1, Solid,\ndefault color";
                strikethrough = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                txt = "Strikethrough, 4, Double,\ndifferent color";
                strikethrough = 4, LineStyle::Double;
                strikethrough_color = colors::RED;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                txt = "Underline, 1, Solid,\ndefault color";
                underline = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                txt = "Underline, 1, Solid,\ndefault color, skip spaces";
                underline = 1, LineStyle::Solid;
                underline_skip = UnderlineSkip::SPACES;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                txt = "Underline, 1, Solid,\ndefault color, descent";
                underline = 1, LineStyle::Solid;
                underline_position = UnderlinePosition::Descent;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                txt = "Underline, 3, wavy,\ndifferent color, no skip";
                underline = 3, LineStyle::Wavy(1.0);
                underline_color = colors::GREEN;
                underline_skip = UnderlineSkip::NONE;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
            }
        ],
    )
}

fn defaults(ctx: &mut WindowContext) -> impl UiNode {
    fn demo(ctx: &mut WindowContext, title: &str, font_family: impl Into<FontNames>) -> impl UiNode {
        let font_family = font_family.into();

        let font = Fonts::req(ctx.services).list(
            &font_family,
            FontStyle::Normal,
            FontWeight::NORMAL,
            FontStretch::NORMAL,
            &lang!(und),
        );

        h_stack! {
            children_align = Align::BASELINE_LEFT;
            children = ui_list![
                text(if title.is_empty() {
                    formatx!("{font_family}: ")
                } else {
                    formatx!("{title}: ")
                }),
                text! {
                    txt = font.best().display_name().to_text();
                    font_family;
                }
            ];
        }
    }

    section(
        "defaults",
        ui_list![
            // Generic
            demo(ctx, "", FontName::serif()),
            demo(ctx, "", FontName::sans_serif()),
            demo(ctx, "", FontName::monospace()),
            demo(ctx, "", FontName::cursive()),
            demo(ctx, "", FontName::fantasy()),
            demo(ctx, "Fallback", "not-a-font-get-fallback"),
            demo(ctx, "UI", FontNames::default())
        ],
    )
}

fn section(header: &'static str, items: impl UiNodeList) -> impl UiNode {
    v_stack! {
        spacing = 5;
        children = ui_list![text! {
            txt = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}
