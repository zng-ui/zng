#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::text::FontsExt;
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("profile-text.json.gz", &[("example", &"text")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        let fs = var(Length::Pt(11.0));
        window! {
            title = fs.map(|s| formatx!("Text Example - font_size: {s}"));
            font_size = fs.clone();
            content = h_stack! {
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
                            line_break(),
                        ];
                    },
                    font_size(fs),
                ];
            };
        }
    })
}

fn font_size(font_size: RcVar<Length>) -> impl Widget {
    fn change_size(font_size: &RcVar<Length>, change: f32, ctx: &mut WidgetContext) {
        font_size.modify(ctx, move |s| {
            **s += Length::Pt(change);
        });
    }
    section(
        "font_size",
        widgets![
            button! {
                content = text("Increase Size");
                on_click = hn!(font_size, |ctx, _| {
                    change_size(&font_size, 1.0, ctx)
                });
            },
            button! {
                content = text("Decrease Size");
                on_click = hn!(font_size, |ctx, _| {
                    change_size(&font_size, -1.0, ctx)
                });
            },
        ],
    )
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
                text = "1.3em: 'Émp Giga Ç'";
                background_color = colors::LIGHT_BLUE;
                color = colors::BLACK;
                line_height = 1.3.em();
            },
        ],
    )
}

fn line_break() -> impl Widget {
    section(
        "line_break",
        widgets![text! {
            text = "Hello line 1!\n    Hello line 2!";
            background_color = rgba(1.0, 1.0, 1.0, 0.3);
        }],
    )
}

fn defaults(ctx: &mut WindowContext) -> impl Widget {
    fn demo(ctx: &mut WindowContext, title: &str, font_family: impl Into<FontNames>) -> impl Widget {
        let font_family = font_family.into();

        let font = ctx.services.fonts().get_list(
            &font_family,
            FontStyle::Normal,
            FontWeight::NORMAL,
            FontStretch::NORMAL,
            &lang!(und),
        );

        h_stack(widgets![
            text(if title.is_empty() {
                formatx!("{font_family}: ")
            } else {
                formatx!("{title}: ")
            }),
            text! {
                text = font.best().display_name().to_text();
                font_family;
            }
        ])
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
