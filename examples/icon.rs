#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use icons::MaterialIcon;
use zero_ui::prelude::*;

use zero_ui_material_icons as icons;
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("icon");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().extend(icons::MaterialFonts).run_window(async {
        Window! {
            title = "Icon Example";
            icon = WindowIcon::render(|| Icon! {
                ico = icons::filled::LIGHTBULB;
                ico_color = colors::YELLOW;
                drop_shadow = (0, 0), 3, colors::WHITE;
            });
            child = Scroll! {
                mode = ScrollMode::VERTICAL;
                child_align = Align::FILL;
                child = icons();
            };
            // zero_ui::properties::inspector::show_hit_test = true;
        }
    })
}

fn icons() -> impl UiNode {
    let selected_font = var("outlined");
    let search = var(Txt::from_static(""));
    fn select_font(key: &'static str) -> impl UiNode {
        Toggle! {
            child = Text!(key);
            value::<&'static str> = key;
        }
    }
    fn show_font(icons: Vec<MaterialIcon>, font_mod: &'static str) -> impl UiNode {
        let _scope = tracing::error_span!("show_font").entered();
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
                                .map(|i| icon_btn(i.clone(), font_mod).boxed())
                                .collect_into_vec(&mut r);
                        r
                    };
                }.boxed())
                .collect_into_vec(&mut r);
                r
            },
        }
    }
    Stack! {
        direction = StackDirection::top_to_bottom();
        padding = 5;
        children_align = Align::TOP;
        children = ui_vec![
            TextInput! {
                txt = search.clone();
                margin = (15, 0, 0, 0);
                padding = (7, 15, 7, 26);
                min_width = 40.vh_pct();
                foreground = Icon! {
                    align = Align::LEFT;
                    ico = icons::outlined::SEARCH;
                    ico_size = 18;
                    margin = (0, 0, 0, 6);
                };
                background = Text! {
                    padding = (7, 15, 7, 26);
                    txt = "search icons";
                    opacity = 50.pct();
                    visibility = search.map(|t| t.is_empty().into());
                };
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
                    select_font("two_tone"),
                ]
            },
            View!(
                ::<(&'static str, Txt)>,
                merge_var!(selected_font, search, |f, s| (*f, s.clone())),
                hn!(|a: &ViewArgs<(&'static str, Txt)>| {
                    if let Some((f, s)) = a.get_new() {
                        let mut icons = match f {
                            "filled" => icons::filled::all(),
                            "outlined" => icons::outlined::all(),
                            "rounded" => icons::rounded::all(),
                            "sharp" => icons::sharp::all(),
                            "two_tone" => icons::two_tone::all(),
                            _ => unreachable!(),
                        };
                        if !s.is_empty() {
                            let s = s.to_uppercase();
                            icons.retain(|f| {
                                f.name.contains(&s) ||
                                f.display_name().to_uppercase().contains(&s)
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

fn icon_btn(ico: icons::MaterialIcon, font_mod: &'static str) -> impl UiNode {
    Button! {
        padding = 2;
        size = (80, 80);
        child = Stack! {
            direction = StackDirection::top_to_bottom();
            spacing = 2;
            children_align = Align::CENTER;
            children = ui_vec![
                Icon! {
                    ico = ico.clone();
                },
                Text! {
                    txt = formatx!("{ico}");
                    txt_wrap = false;
                    font_size = 10;
                },
            ]
        };
        on_click = hn!(|_| {
            LAYERS.insert(LayerIndex::TOP_MOST, expanded_icon(ico.clone(), font_mod));
        })
    }
}

fn expanded_icon(ico: icons::MaterialIcon, font_mod: &'static str) -> impl UiNode {
    let opacity = var(0.fct());
    opacity.ease(1.fct(), 200.ms(), easing::linear).perm();
    Container! {
        opacity = opacity.clone();

        id = "expanded-icon";
        modal = true;
        background_color = color_scheme_map(colors::WHITE.with_alpha(10.pct()), colors::BLACK.with_alpha(10.pct()));
        child_align = Align::CENTER;
        on_click = hn!(|args: &ClickArgs| {
            if WIDGET.id() == args.target.widget_id() {
                LAYERS.remove("expanded-icon");
                args.propagation().stop();
            }
        });
        child = Container! {
            id = "panel";
            background_color = color_scheme_map(colors::BLACK.with_alpha(90.pct()), colors::WHITE.with_alpha(90.pct()));
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
                        title(formatx!("{ico}")),
                        {
                            let full_path = formatx!("zero_ui_material_icons::{font_mod}::{}", ico.name);
                            let copied = var(false);
                            Text! {
                                txt = ico.name;
                                font_family = FontName::monospace();
                                font_size = 18;
                                cursor = CursorIcon::Hand;
                                enabled = copied.map(|&c| !c);
                                tooltip = Tip!(Text!("copy '{full_path}'"));
                                disabled_tooltip = Tip!(Text!("copied!"));
                                on_click = async_hn!(copied, full_path, |_| {
                                    zero_ui::core::clipboard::CLIPBOARD.set_text(&full_path);
                                    copied.set(true);
                                    task::deadline(2.secs()).await;
                                    copied.set(false);
                                });
                                when *#is_hovered {
                                    background_color = FONT_COLOR_VAR.map(|c| c.with_alpha(20.pct()));
                                }
                            }
                        },
                        sub_title("Using `Icon!`:"),
                        Stack! {
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

                                            background_color =  color_scheme_map(
                                                colors::BLACK.with_alpha(85.pct()),
                                                colors::WHITE.with_alpha(85.pct())
                                            );
                                            corner_radius = 4;
                                            padding = 2;
                                        }
                                    ]
                                }.boxed()
                            })).collect::<Vec<_>>()
                        },

                        sub_title("Using `Text!`:"),
                        Stack! {
                            direction = StackDirection::left_to_right();
                            spacing = 5;
                            children_align = Align::TOP_LEFT;
                            children = [64, 48, 32, 24, 16].into_iter().map(clmv!(ico, |size| {
                                Stack! {
                                    direction = StackDirection::top_to_bottom();
                                    spacing = 3;
                                    children = ui_vec![
                                        size_label(formatx!("{size}")),
                                        Text! {
                                            txt = ico.code;
                                            font_family = ico.font.clone();
                                            font_size = size;

                                            background_color = color_scheme_map(
                                                colors::BLACK.with_alpha(85.pct()),
                                                colors::WHITE.with_alpha(85.pct())
                                            );
                                            corner_radius = 4;
                                            padding = 2;
                                        }
                                    ]
                                }.boxed()
                            })).collect::<Vec<_>>()
                        }
                    ]
                },
                Button! {
                    id = "close-btn";
                    icon::ico_size = 14;
                    child = Icon!(icons::filled::CLOSE);
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

fn title(title: Txt) -> impl UiNode {
    Text! {
        txt = title;
        font_size = 24;
        txt_align = Align::CENTER;
    }
}
fn sub_title(title: impl Into<Txt>) -> impl UiNode {
    Text! {
        txt = title.into();
        font_size = 16;
    }
}
fn size_label(size: Txt) -> impl UiNode {
    Text! {
        txt = size;
        font_size = 10;
        txt_align = Align::CENTER;
    }
}
