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
    App::default().extend(icons::MaterialFonts).run_window(|_| {
        window! {
            title = "Icon Example";
            icon = WindowIcon::render(|_| icon! {
                ico = icons::filled::LIGHTBULB;
                ico_color = colors::YELLOW;
                drop_shadow = (0, 0), 3, colors::WHITE;
            });
            child = scroll! {
                mode = ScrollMode::VERTICAL;
                child = icons();
            };
            // zero_ui::properties::inspector::show_hit_test = true;
        }
    })
}

fn icons() -> impl UiNode {
    let selected_font = var("outlined");
    fn select_font(key: &'static str) -> impl UiNode {
        toggle! {
            child = text(key);
            value::<&'static str> = key;
        }
    }
    fn show_font(icons: Vec<MaterialIcon>) -> impl UiNode {
        // let start = std::time::Instant::now();
        wrap! {
            spacing = 5;
            // zero_ui::properties::events::widget::on_info_init = hn_once!(|_, _| {
            //     println!("INIT: {:?}", start.elapsed());
            // });
            icon::vis::ico_size = 48;
            children = icons.into_iter()
                    .map(|i| icon_btn(i).boxed())
                    .collect::<Vec<_>>(),
        }
    }
    v_stack! {
        padding = (20, 5, 5, 5);
        spacing = 20;
        children_align = Align::TOP;
        children = ui_list![
            h_stack! {
                toggle::selector = toggle::Selector::single(selected_font.clone());
                spacing = 5;
                children = ui_list![
                    select_font("filled"),
                    select_font("outlined"),
                    select_font("rounded"),
                    select_font("sharp"),
                    select_font("two_tone"),
                ]
            },
            view(selected_font, show_font(icons::outlined::all()), |ctx, font| {
                match font.get_new(ctx) {
                    Some("filled") => View::Update(show_font(icons::filled::all())),
                    Some("outlined") => View::Update(show_font(icons::outlined::all())),
                    Some("rounded") => View::Update(show_font(icons::rounded::all())),
                    Some("sharp") => View::Update(show_font(icons::sharp::all())),
                    Some("two_tone") => View::Update(show_font(icons::two_tone::all())),
                    None => View::Same,
                    Some(_) => unreachable!(),
                }
            }),
        ]
    }
}

fn icon_btn(ico: icons::MaterialIcon) -> impl UiNode {
    button! {
        padding = 2;
        size = (80, 80);
        child = v_stack! {
            spacing = 2;
            children_align = Align::CENTER;
            children = ui_list![
                icon! {
                    ico = ico.clone();
                },
                text! {
                    txt = formatx!("{ico}");
                    font_size = 10;
                },
            ]
        };
        on_click = hn!(|ctx, _| {
            WindowLayers::insert(
                ctx,
                LayerIndex::TOP_MOST,
                expanded_icon(ctx.vars, ico.clone())
            );
        })
    }
}

fn expanded_icon(vars: &Vars, ico: icons::MaterialIcon) -> impl UiNode {
    let opacity = var(0.fct());
    opacity.ease(vars, 1.fct(), 200.ms(), easing::linear).perm();
    container! {
        opacity = opacity.clone();

        id = "expanded-icon";
        modal = true;
        background_color = color_scheme_map(colors::WHITE.with_alpha(10.pct()), colors::BLACK.with_alpha(10.pct()));
        child_align = Align::CENTER;
        on_click = hn!(|ctx, args: &ClickArgs| {
            if ctx.path.widget_id() == args.target.widget_id() {
                WindowLayers::remove(ctx, "expanded-icon");
                args.propagation().stop();
            }
        });
        child = container! {
            id = "panel";
            background_color = color_scheme_map(colors::BLACK.with_alpha(90.pct()), colors::WHITE.with_alpha(90.pct()));
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            drop_shadow = (0, 0), 4, colors::BLACK;
            child = z_stack(ui_list![
                v_stack! {
                    spacing = 5;
                    padding = 10;
                    children_align = Align::TOP_LEFT;
                    children = ui_list![
                        title(formatx!("{ico}")),
                        text! {
                            txt = ico.name;
                            font_family = FontName::monospace();
                            font_size = 18;
                        },
                        sub_title("Using `icon!`:"),
                        h_stack! {
                            spacing = 5;
                            children_align = Align::TOP_LEFT;
                            children = [64, 48, 32, 24, 16].into_iter().map(clone_move!(ico, |size| {
                                v_stack! {
                                    spacing = 3;
                                    children = ui_list![
                                        size_label(formatx!("{size}")),
                                        icon! {
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

                        sub_title("Using `text!`:"),
                        h_stack! {
                            spacing = 5;
                            children_align = Align::TOP_LEFT;
                            children = [64, 48, 32, 24, 16].into_iter().map(clone_move!(ico, |size| {
                                v_stack! {
                                    spacing = 3;
                                    children = ui_list![
                                        size_label(formatx!("{size}")),
                                        text! {
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
                button! {
                    id = "close-btn";
                    icon::ico_size = 14;
                    child = icon(icons::filled::CLOSE);
                    align = Align::TOP_RIGHT;
                    padding = 2;
                    margin = 4;
                    on_click = async_hn!(opacity, |ctx, args: ClickArgs| {
                        args.propagation().stop();

                        opacity.ease(&ctx, 0.fct(), 150.ms(), easing::linear).perm();
                        ctx.yield_one().await;
                        opacity.wait_animation().await;

                       ctx.with(|ctx|  WindowLayers::remove(ctx, "expanded-icon"));
                    });
                }
            ])
        }
    }
}

fn title(title: Text) -> impl UiNode {
    text! {
        txt = title;
        font_size = 24;
        txt_align = TextAlign::CENTER;
    }
}
fn sub_title(title: impl Into<Text>) -> impl UiNode {
    text! {
        txt = title.into();
        font_size = 16;
    }
}
fn size_label(size: Text) -> impl UiNode {
    text! {
        txt = size;
        font_size = 10;
        txt_align = TextAlign::CENTER;
    }
}
