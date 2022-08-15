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

        let actual_fs = fs.deep_clone().easing(150.ms(), easing::linear);
        fs.bind(ctx, &actual_fs).perm();

        window! {
            title = fs.map(|s| formatx!("Text Example - font_size: {s}"));
            font_size = actual_fs;
            content = z_stack(widgets![
                h_stack! {
                    align = Align::CENTER;
                    spacing = 40;
                    items = widgets![
                        v_stack! {
                            spacing = 20;
                            items = widgets![
                                basic(),
                                defaults(ctx),
                            ];
                        },
                        v_stack! {
                            spacing = 20;
                            items = widgets![
                                line_height(),
                                line_spacing(),
                                word_spacing(),
                                letter_spacing(),
                            ];
                        },
                        v_stack! {
                            spacing = 20;
                            items = widgets![
                                decoration_lines(),
                            ]
                        }
                    ];
                },
                container! {
                    align = Align::TOP;
                    margin = 10;
                    content = font_size(fs);
                },
            ])
        }
    })
}

fn font_size(font_size: RcVar<Length>) -> impl Widget {
    fn change_size(font_size: &RcVar<Length>, change: f32, ctx: &mut WidgetContext) {
        font_size.modify(ctx, move |mut s| {
            *s += Length::Pt(change);
        });
    }
    h_stack! {
        button::vis::dark = theme_generator!(|_| {
            button::vis::dark_theme! {
                padding = (0, 5);
            }
        });
        button::vis::light = theme_generator!(|_| {
            button::vis::light_theme! {
                padding = (0, 5);
            }
        });
        spacing = 5;
        corner_radius = 4;
        background_color = rgba(0, 0, 0, 40.pct());
        padding = 4;
        items = widgets![
            button! {
                content = text("-");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!(Minus), shortcut!(NumpadSubtract)];
                on_click = hn!(font_size, |ctx, _| {
                    change_size(&font_size, -1.0, ctx)
                });
            },
            text! {
                text = font_size.map(|s| formatx!("{s}"));
            },
            button! {
                content = text("+");
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

fn basic() -> impl Widget {
    section(
        "basic",
        widgets![
            text("Basic Text"),
            strong("Strong Text"),
            em("Emphasis Text"),
            text! {
                color = colors::LIGHT_GREEN;
                text = "Colored Text";

                when self.is_hovered {
                    color = colors::YELLOW;
                }
            },
        ],
    )
}

fn line_height() -> impl Widget {
    section(
        "line_height",
        widgets![
            text! {
                text = "Default: 'Émp Giga Ç'";
                background_color = colors::LIGHT_BLUE;
                color = colors::BLACK;
            },
            text! {
                text = "150%: 'Émp Giga Ç'";
                background_color = colors::LIGHT_BLUE;
                color = colors::BLACK;
                line_height = 150.pct();
            },
        ],
    )
}

fn line_spacing() -> impl Widget {
    section(
        "line_spacing",
        widgets![container! {
            content = text! {
                text = "Hello line 1!\nHello line 2!\nHover to change `line_spacing`";
                background_color = rgba(0.5, 0.5, 0.5, 0.3);

                when self.is_hovered {
                    line_spacing = 30.pct();
                }
            };
            content_align = Align::TOP;
            min_height = 1.7.em() * 3.fct();
        }],
    )
}

fn word_spacing() -> impl Widget {
    section(
        "word_spacing",
        widgets![text! {
            text = "Word spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when self.is_hovered {
                word_spacing = 100.pct();
            }
        }],
    )
}

fn letter_spacing() -> impl Widget {
    section(
        "letter_spacing",
        widgets![text! {
            text = "Letter spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when self.is_hovered {
                letter_spacing = 30.pct();
            }
        }],
    )
}

fn decoration_lines() -> impl Widget {
    section(
        "Decorations",
        widgets![
            text! {
                text = "Overline, 1, Dotted,\ndefault color";
                overline = 1, LineStyle::Dotted;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                text = "Strikethrough, 1, Solid,\ndefault color";
                strikethrough = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                text = "Strikethrough, 4, Double,\ndifferent color";
                strikethrough = 4, LineStyle::Double;
                strikethrough_color = colors::RED;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                text = "Underline, 1, Solid,\ndefault color";
                underline = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                text = "Underline, 1, Solid,\ndefault color, skip spaces";
                underline = 1, LineStyle::Solid;
                underline_skip = UnderlineSkip::SPACES;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                text = "Underline, 1, Solid,\ndefault color, descent";
                underline = 1, LineStyle::Solid;
                underline_position = UnderlinePosition::Descent;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            text! {
                text = "Underline, 3, wavy,\ndifferent color, no skip";
                underline = 3, LineStyle::Wavy(1.0);
                underline_color = colors::GREEN;
                underline_skip = UnderlineSkip::NONE;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
            }
        ],
    )
}

fn defaults(ctx: &mut WindowContext) -> impl Widget {
    fn demo(ctx: &mut WindowContext, title: &str, font_family: impl Into<FontNames>) -> impl Widget {
        let font_family = font_family.into();

        let font = Fonts::req(ctx.services).list(
            &font_family,
            FontStyle::Normal,
            FontWeight::NORMAL,
            FontStretch::NORMAL,
            &lang!(und),
        );

        h_stack! {
            items_align = Align::BASELINE_LEFT;
            items = widgets![
                text(if title.is_empty() {
                    formatx!("{font_family}: ")
                } else {
                    formatx!("{title}: ")
                }),
                text! {
                    text = font.best().display_name().to_text();
                    font_family;
                }
            ];
        }
    }

    section(
        "defaults",
        widgets![
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

fn section(header: &'static str, items: impl WidgetList) -> impl Widget {
    v_stack! {
        spacing = 5;
        items = widgets![text! {
            text = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }].chain(items);
    }
}
