#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_material_icons as icons;
use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("profile-icon.json.gz", &[("example", &"icon")], |_| true);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().extend(icons::MaterialFonts).run_window(|_| {
        window! {
            title = "Icon Example";
            content = scrollable! {
                mode = ScrollMode::VERTICAL;
                content = uniform_grid! {
                    padding = 5;
                    spacing = 5;
                    columns = 5;

                    icon::theme::icon_size = 48;
                    items = icons::outlined::all().into_iter().take(50)
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
        content = v_stack! {
            spacing = 2;
            items_align = Align::CENTER;
            items = widgets![
                icon(ico.clone()),
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
        on_click = hn!(|ctx, _| {
            WindowLayers::remove(ctx, "expanded-icon");
        });
        content = container! {
            background_color = colors::BLACK.with_alpha(90.pct());
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            drop_shadow = (0, 0), 4, colors::BLACK;
            padding = 10;
            content = v_stack! {
                spacing = 5;
                items_align = Align::TOP_LEFT;
                items = widgets![
                    text! {
                        text = formatx!("{ico}");
                        font_size = 24;
                    },
                    icon! {
                        icon = ico;
                        size = 64;
                    }
                ]
            }
        }
    }
}
