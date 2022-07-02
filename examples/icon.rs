#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_material_icons as icons;
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    // examples_util::print_info();
    zero_ui_view::init();

    let rec = examples_util::record_profile("profile-icon.json.gz", &[("example", &"icon")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    rec.finish();
}

fn app_main() {
    App::default().extend(icons::MaterialFonts).run_window(|_| {
        window! {
            title = "Icon Example";
            icon = WindowIcon::render(RenderMode::Software, |_| icon! {
                icon = icons::filled::LIGHTBULB;
                color = colors::YELLOW;
                drop_shadow = (0, 0), 3, colors::WHITE;
            });
            content = scroll! {
                mode = ScrollMode::VERTICAL;
                content = wrap! {
                    padding = 10;
                    spacing = 5;
                    icon::theme::icon_size = 48;
                    items = icons::outlined::all().into_iter()
                            .map(|i| icon_btn(i).boxed_wgt())
                            .collect::<WidgetVec>(),
                }
            };
        }
    })
}

fn icon_btn(ico: icons::MaterialIcon) -> impl Widget {
    button! {
        padding = 2;
        size = (80, 80);
        content = v_stack! {
            spacing = 2;
            items_align = Align::CENTER;
            items = widgets![
                icon! {
                    icon = ico.clone();
                },
                text! {
                    text = formatx!("{ico}");
                    font_size = 10;
                },
            ]
        };
        on_click = hn!(|ctx, _| {
            WindowLayers::insert(
                ctx,
                LayerIndex::TOP_MOST,
                expanded_icon(ico.clone())
            );
        })
    }
}

fn expanded_icon(ico: icons::MaterialIcon) -> impl Widget {
    container! {
        id = "expanded-icon";
        modal = true;
        background_color = colors::WHITE.with_alpha(10.pct());
        content_align = Align::CENTER;
        on_click = hn!(|ctx, args: &ClickArgs| {
            if ctx.path.widget_id() == args.target.widget_id() {
                WindowLayers::remove(ctx, "expanded-icon");
                args.propagation().stop();
            }
        });
        content = container! {
            id = "panel";
            background_color = rgb(0.1, 0.1, 0.1);
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            drop_shadow = (0, 0), 4, colors::BLACK;
            content = z_stack(widgets![
                v_stack! {
                    spacing = 5;
                    padding = 10;
                    items_align = Align::TOP_LEFT;
                    items = widgets![
                        title(formatx!("{ico}")),
                        text! {
                            text = ico.name;
                            font_family = FontName::monospace();
                            font_size = 18;
                            color = colors::WHITE.darken(4.pct());
                        },
                        sub_title("Using `icon!`:"),
                        h_stack! {
                            spacing = 5;
                            items_align = Align::TOP_LEFT;
                            items = [64, 48, 32, 24, 16].into_iter().map(clone_move!(ico, |size| {
                                v_stack! {
                                    spacing = 3;
                                    items = widgets![
                                        size_label(formatx!("{size}")),
                                        icon! {
                                            icon = ico.clone();
                                            icon_size = size;

                                            background_color = rgb(0.15, 0.15, 0.15);
                                            corner_radius = 4;
                                            padding = 2;
                                        }
                                    ]
                                }.boxed_wgt()
                            })).collect::<WidgetVec>()
                        },

                        sub_title("Using `text!`:"),
                        h_stack! {
                            spacing = 5;
                            items_align = Align::TOP_LEFT;
                            items = [64, 48, 32, 24, 16].into_iter().map(clone_move!(ico, |size| {
                                v_stack! {
                                    spacing = 3;
                                    items = widgets![
                                        size_label(formatx!("{size}")),
                                        text! {
                                            text = ico.code;
                                            font_family = ico.font.clone();
                                            font_size = size;

                                            background_color = rgb(0.15, 0.15, 0.15);
                                            corner_radius = 4;
                                            padding = 2;
                                        }
                                    ]
                                }.boxed_wgt()
                            })).collect::<WidgetVec>()
                        }
                    ]
                },
                button! {
                    id = "close-btn";
                    icon::theme::icon_size = 14;
                    content = icon(icons::filled::CLOSE);
                    align = Align::TOP_RIGHT;
                    padding = 2;
                    margin = 4;
                    on_click = hn!(|ctx, args: &ClickArgs| {
                        WindowLayers::remove(ctx, "expanded-icon");
                        args.propagation().stop();
                    });
                }
            ])
        }
    }
}

fn title(title: Text) -> impl Widget {
    text! {
        text = title;
        font_size = 24;
        text_align = TextAlign::CENTER;
    }
}
fn sub_title(title: impl Into<Text>) -> impl Widget {
    text! {
        text = title.into();
        font_size = 16;
    }
}
fn size_label(size: Text) -> impl Widget {
    text! {
        text = size;
        font_size = 10;
        text_align = TextAlign::CENTER;
    }
}
