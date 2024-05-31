//! Demonstrates borders, corner radius, multiple borders per widget and clip-to-bounds.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zng::{
    prelude::*,
    widget::{background_color, border_align, corner_radius},
};

fn main() {
    examples_util::print_info();
    zng::env::init!();
    zng::app::crash_handler::init_debug();

    //let rec = examples_util::record_profile("border");

    // zng::view_process::prebuilt::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    APP.defaults().run_window(async {
        Window! {
            title = "Border Example";

            background_color = web_colors::BLUE.darken(70.pct());

            color_scheme = color::ColorScheme::Dark;

            child = Stack! {
                layout::align = Align::CENTER;
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
        widget::border = 0.5, web_colors::RED.darken(20.pct());
        clip_to_bounds = clip.clone();
        gesture::on_click = hn!(clip, |_| {
            clip.modify(|c| *c.to_mut() = !**c)
        });
        child = Text! {
            corner_radius = 0;
            background_color = web_colors::GREEN.darken(40.pct());
            layout::padding = 3;
            layout::rotate = -(5.deg());
            txt_align = Align::CENTER;
            txt = clip.map(|c| formatx!("clip_to_bounds = {c}"));
        };
    }
}

mod widgets {
    use zng::{
        container::Container,
        prelude_wgt::*,
        widget::{self, border},
    };

    #[widget($crate::widgets::MrBorders)]
    pub struct MrBorders(Container);
    impl MrBorders {
        fn widget_intrinsic(&mut self) {
            widget_set! {
                self;
                padding = 20;

                child_align = Align::CENTER;

                widget::background_color = web_colors::GREEN.darken(40.pct());

                border0 = 4, colors::WHITE.with_alpha(20.pct());
                border1 = 4, colors::BLACK.with_alpha(20.pct());
                border2 = 4, colors::WHITE.with_alpha(20.pct());

                widget::foreground_highlight = 3, 1, web_colors::ORANGE;

                widget::corner_radius = 20;
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
