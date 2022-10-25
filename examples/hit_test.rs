use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();

    zero_ui_view::init();

    App::default().run_window(|_| {
        window! {
            title = "Hit-Test Example";

            child_align = Align::CENTER;
            child = h_stack! {
                spacing = 14;
                children = ui_list![
                    example(HitTestMode::Visual),
                    example(HitTestMode::RoundedBounds),
                    example(HitTestMode::Bounds),
                ]
            };
        }
    })
}

fn example(mode: HitTestMode) -> impl UiNode {
    container! {
        hit_test_mode = mode;

        on_click = hn!(mode, |_, _| {
            println!("Clicked {:?}", mode);
        });

        child = text(formatx!("{:#?}", mode));
        padding = 40;
        corner_radius = 40;
        // background_color = colors::GRAY;

        border = 5, colors::RED;
        when *#is_hovered {
            border = 5, colors::GREEN;
        }
    }
}
