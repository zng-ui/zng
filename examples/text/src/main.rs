//! Demonstrates the `Text!` and `TextInput!` widgets. Text rendering, text editor.

use zng::{
    button,
    font::{FontName, FontNames},
    gesture::{click_shortcut, is_hovered},
    layout::{align, margin, padding},
    prelude::*,
    text::{UnderlinePosition, UnderlineSkip, font_family, font_weight},
    widget::{LineStyle, background_color, corner_radius},
};

mod editor;
mod form;

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        let fs = var(Length::Pt(11.0));

        Window! {
            title = fs.map(|s| formatx!("Text Example - font_size: {s}"));
            child = Stack!(ui_vec![
                Stack! {
                    text::font_size = fs.easing(150.ms(), easing::linear);
                    direction = StackDirection::left_to_right();
                    align = Align::CENTER;
                    spacing = 40;
                    children = ui_vec![
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![basic(), defaults()];
                        },
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![line_height(), line_spacing(), word_spacing(), letter_spacing()];
                        },
                        Stack! {
                            direction = StackDirection::top_to_bottom();
                            spacing = 20;
                            children = ui_vec![decoration_lines()]
                        }
                    ];
                },
                Container! {
                    align = Align::TOP;
                    margin = 10;
                    child = font_size_example(fs);
                },
                Stack! {
                    direction = StackDirection::top_to_bottom();
                    spacing = 5;
                    margin = 20;
                    align = Align::BOTTOM_RIGHT;
                    children_align = Align::RIGHT;
                    children = ui_vec![editor::text_editor(), form::form_editor()];
                },
            ])
        }
    })
}

fn font_size_example(font_size: Var<Length>) -> UiNode {
    fn change_size(font_size: &Var<Length>, change: f32) {
        font_size.modify(move |s| {
            **s += Length::Pt(change);
        });
    }
    Stack! {
        button::style_fn = Style! {
            padding = (0, 5)
        };
        direction = StackDirection::left_to_right();
        spacing = 5;
        corner_radius = 4;
        background_color = light_dark(rgba(1., 1., 1., 40.pct()), rgba(0, 0, 0, 40.pct()));
        padding = 4;
        children = ui_vec![
            Button! {
                child = Text!("-");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!('-')];
                on_click = hn!(font_size, |_| change_size(&font_size, -1.0));
            },
            Text! {
                txt = font_size.map(|s| formatx!("{s}"));
            },
            Button! {
                child = Text!("+");
                font_family = FontName::monospace();
                font_weight = FontWeight::BOLD;
                click_shortcut = [shortcut!('+')];
                on_click = hn!(font_size, |_| change_size(&font_size, 1.0));
            },
        ]
    }
}

fn basic() -> UiNode {
    section(
        "basic",
        ui_vec![
            Text!("Basic Text"),
            text::Strong!("Strong Text"),
            text::Em!("Emphasis Text"),
            Text! {
                font_color = light_dark(web_colors::DARK_GREEN, web_colors::LIGHT_GREEN);
                txt = "Colored Text";

                when *#is_hovered {
                    font_color = light_dark(web_colors::BROWN, web_colors::YELLOW);
                }
            },
            Text!("Emoticons 🔎👨‍💻🧐"),
        ],
    )
}

fn line_height() -> UiNode {
    section(
        "line_height",
        ui_vec![
            Text! {
                txt = "Default: 'Émp Giga Ç'";
                background_color = web_colors::LIGHT_BLUE;
                font_color = web_colors::BLACK;
            },
            Text! {
                txt = "150%: 'Émp Giga Ç'";
                background_color = web_colors::LIGHT_BLUE;
                font_color = web_colors::BLACK;
                line_height = 150.pct();
            },
        ],
    )
}

fn line_spacing() -> UiNode {
    section(
        "line_spacing",
        ui_vec![Container! {
            child = Text! {
                txt = "Hello line 1!\nHello line 2!\nHover to change `line_spacing`";
                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                txt_wrap = false;

                when *#is_hovered {
                    #[easing(150.ms())]
                    line_spacing = 30.pct();
                }
            };
            child_align = Align::TOP;
            layout::min_height = 1.7.em() * 3.fct();
        }],
    )
}

fn word_spacing() -> UiNode {
    section(
        "word_spacing",
        ui_vec![Text! {
            txt = "Word spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                #[easing(150.ms())]
                word_spacing = 100.pct();
            }
        }],
    )
}

fn letter_spacing() -> UiNode {
    section(
        "letter_spacing",
        ui_vec![Text! {
            txt = "Letter spacing\n\thover to change";
            background_color = rgba(0.5, 0.5, 0.5, 0.3);

            when *#is_hovered {
                #[easing(150.ms())]
                letter_spacing = 30.pct();
            }
        }],
    )
}

fn decoration_lines() -> UiNode {
    section(
        "Decorations",
        ui_vec![
            Text! {
                txt = "Overline, 1, Dotted,\ndefault color";
                overline = 1, LineStyle::Dotted;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Strikethrough, 1, Solid,\ndefault color";
                strikethrough = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Strikethrough, 4, Double,\ndifferent color";
                strikethrough = 4, LineStyle::Double;
                strikethrough_color = web_colors::RED;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color";
                underline = 1, LineStyle::Solid;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color, skip spaces";
                underline = 1, LineStyle::Solid;
                underline_skip = UnderlineSkip::SPACES;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 1, Solid,\ndefault color, descent";
                underline = 1, LineStyle::Solid;
                underline_position = UnderlinePosition::Descent;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
                margin = (0, 0, 4, 0);
            },
            Text! {
                txt = "Underline, 3, wavy,\ndifferent color, no skip";
                underline = 3, LineStyle::Wavy(1.0);
                underline_color = web_colors::GREEN;
                underline_skip = UnderlineSkip::NONE;

                background_color = rgba(0.5, 0.5, 0.5, 0.3);
            }
        ],
    )
}

fn defaults() -> UiNode {
    fn demo(title: &str, font_family: impl Into<FontNames>) -> UiNode {
        let font_family = font_family.into();

        let font_name = zng::font::FONTS
            .list(
                &font_family,
                FontStyle::Normal,
                FontWeight::NORMAL,
                FontStretch::NORMAL,
                &lang!(und),
            )
            .map(|f| match f.done() {
                Some(f) => f.best().family_name().to_txt(),
                None => Txt::from_str(""),
            });

        Stack! {
            direction = StackDirection::left_to_right();
            children_align = Align::BASELINE_LEFT;
            children = ui_vec![
                Text!(if title.is_empty() {
                    formatx!("{font_family}: ")
                } else {
                    formatx!("{title}: ")
                }),
                Text! {
                    txt = font_name;
                    font_family;
                    layout::max_width = 200;
                }
            ];
        }
    }

    section(
        "defaults",
        ui_vec![
            // Generic
            demo("", FontName::serif()),
            demo("", FontName::sans_serif()),
            demo("", FontName::monospace()),
            demo("", FontName::cursive()),
            demo("", FontName::fantasy()),
            demo("Fallback", "not-a-font-get-fallback"),
            demo("UI", FontNames::default())
        ],
    )
}

fn section(header: &'static str, items: impl IntoUiNode) -> UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        children = ui_vec![Text! {
            txt = header;
            font_weight = FontWeight::BOLD;
            margin = (0, 4);
        }]
        .chain(items);
    }
}
