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
    App::default().run_window(|_| {
        window! {
            title = "Icon Example";
            content = scrollable! {
                mode = ScrollMode::VERTICAL;
                content = uniform_grid! {
                    padding = 5;
                    spacing = 5;
                    columns = 5;

                    items = icons::outlined::all().into_iter()
                            .map(|i| icon_btn(i).boxed_wgt())
                            .collect::<WidgetVec>(),
                }
            };
        }
    })
}

fn icon_btn(ico: icons::MaterialIcon) -> impl Widget {
    let name = ico.name;
    button! {
        content = v_stack! {
            spacing = 3;
            items = widgets![
                icon(ico),
                text! {
                    text = name;
                    font_size = 10;
                },
            ]
        }
    }
}
