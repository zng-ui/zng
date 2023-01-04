#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();
    app_main();
}

fn app_main() {
    App::default().run_window(|_| {
        window! {
            padding = 10;
            child = stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 20;
                children = ui_vec![
                    // wrap_blocks(colors::BEIGE),
                    nested_wrap(),
                ]
            };
        }
    })
}

fn wrap_blocks(color: Rgba) -> impl UiNode {
    wrap! {
        // txt_align = Align::RIGHT;
        background_color = color;
        children = (0..1).map(|i| text! {
            txt = formatx!("{i}");
            txt_align = Align::CENTER;
            size = (40, 40);
            background_color = if i % 2 == 0 { colors::GRAY } else { colors::BLACK }.with_alpha(50.pct());
        }.boxed()).collect::<UiNodeVec>()
    }
}

fn nested_wrap() -> impl UiNode {
    wrap! {
        id = "outer-wrap";
        children = (0..5).map(|i| wrap_blocks(if i % 2 == 0 {
            colors::DARK_BLUE
        } else {
            colors::DARK_GREEN
        }).boxed())
        .collect::<UiNodeVec>()
    }
}
