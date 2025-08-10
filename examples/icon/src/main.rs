//! Search and copy Material Icons keys.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zng::{
    access::ACCESS,
    clipboard,
    color::filter::{backdrop_blur, drop_shadow, opacity},
    container,
    data_view::{DataView, DataViewArgs},
    focus::{DirectionalNav, TabNav, directional_nav, focus_scope, focus_shortcut, tab_nav},
    font::FontName,
    gesture::{ClickArgs, on_click},
    icon::{self, GlyphIcon, Icon},
    layout::{align, margin, padding},
    prelude::*,
    scroll::{LazyMode, ScrollMode, lazy},
    widget::{background_color, corner_radius},
    wrap,
};

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        Window! {
            title = "Icon Example";
            icon = WindowIcon::render(|| {
                Icon! {
                    ico = icon::material::filled::req("lightbulb");
                    ico_color = colors::YELLOW;
                    ico_size = 48;
                    drop_shadow = (0, 0), 3, colors::WHITE;
                }
            });
            child = Scroll! {
                mode = ScrollMode::VERTICAL;
                child_align = Align::FILL;
                child = icons();
            };
            directional_nav = DirectionalNav::Contained;
        }
    })
}

fn icons() -> UiNode {
    let selected_font = var("outlined");
    let search = var(Txt::from_static(""));
    fn select_font(key: &'static str) -> UiNode {
        Toggle! {
            child = Text!(key);
            value::<&'static str> = key;
        }
    }
    fn show_font(icons: Vec<(&'static str, GlyphIcon)>, font_mod: &'static str) -> UiNode {
        let _scope = tracing::error_span!("show_font").entered();
        let icons_len = icons.len();
        Wrap! {
            spacing = 5;
            icon::ico_size = 48;
            children_align = Align::CENTER;
            children = {
                let mut r = vec![];
                icons
                .par_chunks(100)
                .map(|c| Wrap! { // segment into multiple inlined lazy inited `Wrap!` for better performance.
                    spacing = 5;
                    children_align = Align::CENTER;
                    lazy = {
                        let len = c.len();
                        LazyMode::lazy(wgt_fn!(|_| {
                            wrap::lazy_size(len, 5, (80, 80))
                        }))
                    };
                    children = {
                        let mut r = vec![];
                        c.par_iter()
                                .map(|(name, ico)| icon_btn(name, ico.clone(), font_mod))
                                .collect_into_vec(&mut r);
                        r
                    };
                })
                .collect_into_vec(&mut r);
                r
            };
            zng::container::child_bottom = Wrap! {
                layout::margin = 30;
                layout::align = Align::CENTER;
                children = ui_vec![
                    Text!("{icons_len} results, "),
                    Button! {
                        style_fn = style_fn!(|_| zng::button::LinkStyle!());
                        child = Text!("back to search");
                        on_click = hn!(|_| {
                            FOCUS.focus_widget("search", false);
                        });
                    },
                ];
            }, 0;
        }
    }
    Stack! {
        direction = StackDirection::top_to_bottom();
        padding = 5;
        children_align = Align::TOP;
        children = ui_vec![
            TextInput! {
                id = "search";
                txt = search.clone();
                focus_shortcut = [shortcut!['S'], shortcut![CTRL + 'F'], shortcut![Find]];
                placeholder_txt = "search icons (S)";
                style_fn = zng::text_input::SearchStyle!();
                layout::min_width = 40.vh_pct();
                layout::margin = (15, 0, 0, 0);
            },
            Stack! {
                margin = (10, 0, 20, 0);
                direction = StackDirection::left_to_right();
                toggle::selector = toggle::Selector::single(selected_font.clone());
                spacing = 5;
                children = ui_vec![
                    select_font("filled"),
                    select_font("outlined"),
                    select_font("rounded"),
                    select_font("sharp"),
                ]
            },
            DataView!(
                ::<(&'static str, Txt)>,
                merge_var!(selected_font, search, |f, s| (*f, s.clone())),
                hn!(|a: &DataViewArgs<(&'static str, Txt)>| {
                    if let Some((f, s)) = a.get_new() {
                        let mut icons: Vec<_> = match f {
                            "filled" => icon::material::filled::all().collect(),
                            "outlined" => icon::material::outlined::all().collect(),
                            "rounded" => icon::material::rounded::all().collect(),
                            "sharp" => icon::material::sharp::all().collect(),
                            _ => unreachable!(),
                        };
                        icons.sort_by_key(|(k, _)| *k);
                        if let Some(len) = s.strip_prefix("-len") {
                            let len: usize = len.trim().parse().unwrap_or(0);
                            icons.retain(|f| {
                                f.0.len() >= len
                            });
                        } else if !s.is_empty() {
                            let s = s.to_lowercase();
                            icons.retain(|f| {
                                f.0.contains(&s)
                            });
                        }
                        if icons.is_empty() {
                            a.set_view(Text! {
                                txt = formatx!("no icons found for '{s}'");
                                margin = (10, 0, 0, 0);
                            })
                        } else {
                            a.set_view(show_font(icons, f));
                        }
                    }
                }),
            ),
        ]
    }
}

fn icon_btn(name: &'static str, ico: icon::GlyphIcon, font_mod: &'static str) -> UiNode {
    Button! {
        padding = 2;
        layout::size = (80, 80);
        child = Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 2;
            children_align = Align::CENTER;
            children = ui_vec![
                Icon! {
                    ico = ico.clone();
                },
                Text! {
                    txt = name;
                    txt_align = Align::CENTER;
                    font_size = 10;
                    layout::height = 2.em();
                    line_height = 1.em();
                },
            ]
        };
        on_click = hn!(|_| {
            LAYERS.insert(LayerIndex::TOP_MOST, expanded_icon(name, ico.clone(), font_mod));
        })
    }
}

fn expanded_icon(name: &'static str, ico: icon::GlyphIcon, font_mod: &'static str) -> UiNode {
    let opacity = var(0.fct());
    opacity.ease(1.fct(), 200.ms(), easing::linear).perm();
    Container! {
        opacity = opacity.clone();

        id = "expanded-icon";
        widget::modal = true;
        zng::focus::return_focus_on_deinit = true;
        backdrop_blur = 2;
        background_color = light_dark(colors::BLACK.with_alpha(10.pct()), colors::WHITE.with_alpha(10.pct()));
        child_align = Align::CENTER;
        on_click = hn!(|args: &ClickArgs| {
            if WIDGET.id() == args.target.widget_id() {
                args.propagation().stop();
                ACCESS.click(WINDOW.id(), "close-btn", true);
            }
        });
        child = Container! {
            id = "panel";
            background_color = light_dark(colors::WHITE.with_alpha(90.pct()), colors::BLACK.with_alpha(90.pct()));
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            drop_shadow = (0, 0), 4, colors::BLACK;
            child = Stack!(children = ui_vec![
                Stack! {
                    direction = StackDirection::top_to_bottom();
                    spacing = 5;
                    padding = 10;
                    children_align = Align::TOP_LEFT;
                    children = ui_vec![
                        title(name.into()),
                        Stack! {
                            align = Align::CENTER;
                            margin = 10;
                            direction = StackDirection::left_to_right();
                            spacing = 5;
                            children_align = Align::TOP_LEFT;
                            children = [64, 48, 32, 24, 16].into_iter().map(clmv!(ico, |size| {
                                Stack! {
                                    direction = StackDirection::top_to_bottom();
                                    spacing = 3;
                                    children = ui_vec![
                                        size_label(formatx!("{size}")),
                                        Icon! {
                                            ico = ico.clone();
                                            ico_size = size;

                                            background_color = light_dark(
                                                colors::WHITE.with_alpha(85.pct()),
                                                colors::BLACK.with_alpha(85.pct()),
                                            );
                                            corner_radius = 4;
                                            padding = 2;
                                        }
                                    ]
                                    }
                            })).collect::<Vec<_>>()
                        },
                        code_copy("ICONS.req".into(), formatx!("ICONS.req(\"material/{font_mod}/{name}\")")),
                        code_copy(formatx!("{font_mod}::req"), formatx!("icon::{font_mod}::req(\"{name}\")")),
                    ]
                },
                Button! {
                    id = "close-btn";
                    icon::ico_size = 14;
                    child = Icon!(icon::material::filled::req("close"));
                    align = Align::TOP_RIGHT;
                    padding = 2;
                    margin = 4;
                    on_click = async_hn!(opacity, |args: ClickArgs| {
                        args.propagation().stop();

                        opacity.ease(0.fct(), 150.ms(), easing::linear).perm();
                        opacity.wait_animation().await;

                        LAYERS.remove("expanded-icon");
                    });
                }
            ])
        }
    }
}

fn title(title: Txt) -> UiNode {
    Text! {
        txt = title;
        font_size = 24;
        txt_align = Align::CENTER;
    }
}

fn size_label(size: Txt) -> UiNode {
    Text! {
        txt = size;
        font_size = 10;
        txt_align = Align::CENTER;
    }
}

fn code_copy(label: Txt, code: Txt) -> UiNode {
    let enabled = var(true);
    let copy_status = var(Txt::from(""));
    Button! {
        style_fn = zng::button::LightStyle!();
        padding = 2;

        container::child_start = ICONS.get("copy"), 4;
        child = Text!(label);

        text::font_family = FontName::monospace();
        mouse::cursor = mouse::CursorIcon::Pointer;
        widget::enabled = enabled.clone();

        tooltip = Tip!(Text!("copy {code}"));
        tip::disabled_tooltip = Tip!(Text!(copy_status.clone()));

        on_click = async_hn!(enabled, code, copy_status, |_| {
            copy_status.set("copying..");
            enabled.set(false);
            ACCESS.show_tooltip(WINDOW.id(), WIDGET.id());

            match clipboard::CLIPBOARD.set_text(code).wait_rsp().await {
                Ok(copied) => {
                    debug_assert!(copied); // no other clipboard request
                    copy_status.set("copied!");
                },
                Err(e) => copy_status.set(formatx!("error: {e}")),
            }

            task::deadline(2.secs()).await;
            enabled.set(true);
        });
        when *#gesture::is_hovered {
            background_color = text::FONT_COLOR_VAR.map(|c| c.with_alpha(20.pct()));
        }
    }
}
