#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("border);

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(async {
        Window! {
            title = "Border Example";

            background_color = colors::BLUE.darken(70.pct());

            color_scheme = ColorScheme::Dark;

            child = Stack! {
                align = Align::CENTER;
                spacing = 20;
                direction = StackDirection::left_to_right();
                children = ui_vec![
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 20;
                        children = ui_vec![
                            widgets::MrBorders! {
                                border_align = 0.pct();
                                child = Text!("border_align = 0.pct();");
                            },
                            widgets::MrBorders! {
                                border_align = (1.0 / 3.0).fct();
                                child = Text!("border_align = (1.0 / 3.0).fct();");
                            },
                            widgets::MrBorders! {
                                border_align = 50.pct();
                                child = Text!("border_align = 50.pct();");
                            },
                            widgets::MrBorders! {
                                border_align = 100.pct();
                                child = Text!("border_align = 100.pct();");
                            },
                        ]
                    },
                    Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 20;
                        children = ui_vec![
                            widgets::MrBorders! {
                                child = Text!("corner_radius = 0;");
                                corner_radius = 0;
                            },
                            widgets::MrBorders! {
                                child = Text!("corner_radius = 40;");
                                corner_radius = 40;
                            },
                            widgets::MrBorders! {
                                border_align = 100.pct();
                                child = widgets::MrBorders! {
                                    border_align = 100.pct();
                                    child = widgets::MrBorders! {
                                        border_align = 100.pct();
                                        child = Text!("Nested");
                                    },
                                },
                            },
                            clip_to_bounds_demo(),
                        ]
                    },
                ]
            };
        }
    })
}

fn clip_to_bounds_demo() -> impl UiNode {
    let clip = var(true);
    Container! {
        child_align = Align::FILL;
        corner_radius = 10;
        border = 0.5, colors::RED.darken(20.pct());
        clip_to_bounds = clip.clone();
        on_click = hn!(clip, |_| {
            clip.modify(|c| *c.to_mut() = !**c)
        });
        child = Text! {
            corner_radius = 0;
            background_color = colors::GREEN.darken(40.pct());
            padding = 3;
            rotate = -(5.deg());
            txt_align = Align::CENTER;
            txt = clip.map(|c| formatx!("clip_to_bounds = {c}"));
        };
    }
}

mod widgets {
    use zero_ui::prelude::new_widget::*;

    #[widget($crate::widgets::MrBorders)]
    pub struct MrBorders(Container);
    impl MrBorders {
        fn widget_intrinsic(&mut self) {
            widget_set! {
                self;
                padding = 20;

                child_align = Align::CENTER;

                background_color = colors::GREEN.darken(40.pct());

                border0 = 4, colors::WHITE.with_alpha(20.pct());
                border1 = 4, colors::BLACK.with_alpha(20.pct());
                border2 = 4, colors::WHITE.with_alpha(20.pct());

                foreground_highlight = 3, 1, colors::ORANGE;

                corner_radius = 20;
            }
        }
    }

    #[property(BORDER, default(0, BorderStyle::Hidden), widget_impl(MrBorders))]
    pub fn border0(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
        border(child, widths, sides)
    }
    #[property(BORDER, default(0, BorderStyle::Hidden), widget_impl(MrBorders))]
    pub fn border1(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
        border(child, widths, sides)
    }
    #[property(BORDER, default(0, BorderStyle::Hidden), widget_impl(MrBorders))]
    pub fn border2(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
        border(child, widths, sides)
    }
}
