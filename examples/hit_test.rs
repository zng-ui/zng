use zero_ui::prelude::*;

fn main() {
    examples_util::print_info();

    zero_ui_view::init();

    App::default().run_window(|_| {
        window! {
            title = "Hit-Test Example";

            content_align = Align::CENTER;
            content = h_stack! {
                spacing = 14;
                items = widgets![
                    example(HitTestMode::Visual),
                    example(HitTestMode::RoundedBounds),
                    example(HitTestMode::Bounds),
                ]
            };
        }
    })
}

fn example(mode: HitTestMode) -> impl Widget {
    container! {
        hit_test_mode = mode;

        on_click = hn!(mode, |_, _| {
            println!("Clicked {:?}", mode);
        });

        content = text(formatx!("{:#?}", mode));
        padding = 40;
        corner_radius = 40;
        // background_color = colors::GRAY;

        border = 5, colors::RED;
        when self.is_hovered {
            border = 5, colors::GREEN;
        }
    }
}
