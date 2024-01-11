#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::{
    mouse::{cursor, CursorIcon},
    prelude::*,
    scroll::cmd::ScrollToMode,
};

use zero_ui_view_prebuilt as zero_ui_view;

use rand::SeedableRng;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("scroll");
    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    APP.defaults().run_window(async {
        let mouse_pan = var(false);
        let smooth_scrolling = var(true);
        Window! {
            title = "Scroll Example";
            child_top = commands(mouse_pan.clone(), smooth_scrolling.clone()), 0;
            child = Scroll! {
                id = "scroll";
                padding = 20;
                widget::background_color = color::color_scheme_map(
                    hex!(#245E81),
                    colors::WHITE.with_alpha(80.pct()).mix_normal(hex!(#245E81))
                );
                smooth_scrolling = smooth_scrolling.map_into();

                mouse_pan;
                when *#mouse_pan {
                    cursor = CursorIcon::Grab;
                }
                when *#mouse_pan && *#gesture::is_pressed {
                    cursor = CursorIcon::Grabbing;
                }

                child = Stack!{
                    direction = StackDirection::top_to_bottom();
                    children_align = Align::LEFT;
                    children = ui_vec![
                        Text! {
                            id = "Lorem 1";
                            txt = "Lorem 1";
                            font_size = 20;
                        },
                        Text!(ipsum()),
                        Text! {
                            id = "Lorem 2";
                            txt = "Lorem 2";
                            font_size = 20;
                        },
                        Text!(ipsum())
                    ];
                }
            };
        }
    })
}

fn commands(mouse_pan: impl Var<bool>, smooth_scrolling: impl Var<bool>) -> impl UiNode {
    use zero_ui::scroll::cmd::*;

    let scope = WidgetId::named("scroll");

    Menu! {
        id = "menu";
        children = ui_vec![
            SubMenu!(
                "Scroll",
                ui_vec![
                    Button!(SCROLL_UP_CMD.scoped(scope)),
                    Button!(SCROLL_DOWN_CMD.scoped(scope)),
                    Button!(SCROLL_LEFT_CMD.scoped(scope)),
                    Button!(SCROLL_RIGHT_CMD.scoped(scope)),
                ]
            ),
            SubMenu!(
                "Page",
                ui_vec![
                    Button!(PAGE_UP_CMD.scoped(scope)),
                    Button!(PAGE_DOWN_CMD.scoped(scope)),
                    Button!(PAGE_LEFT_CMD.scoped(scope)),
                    Button!(PAGE_RIGHT_CMD.scoped(scope)),
                ]
            ),
            SubMenu!(
                "Scroll to",
                ui_vec![
                    Button!(SCROLL_TO_TOP_CMD.scoped(scope)),
                    Button!(SCROLL_TO_BOTTOM_CMD.scoped(scope)),
                    Button!(SCROLL_TO_LEFTMOST_CMD.scoped(scope)),
                    Button!(SCROLL_TO_RIGHTMOST_CMD.scoped(scope)),
                    Hr!(),
                    scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::minimal(10)),
                    scroll_to_btn(WidgetId::named("Lorem 2"), ScrollToMode::center()),
                    scroll_to_rect((5, 5).at(0.pct(), 50.pct()), ScrollToMode::minimal(10)),
                    scroll_to_rect((5, 5).at(0.pct(), 50.pct()), ScrollToMode::center()),
                ]
            ),
            SubMenu!(
                "Zoom",
                ui_vec![
                    Button!(ZOOM_IN_CMD.scoped(scope)),
                    Button!(ZOOM_OUT_CMD.scoped(scope)),
                    Button!(ZOOM_RESET_CMD.scoped(scope)),
                    Hr!(),
                    scroll_to_zoom_btn(WidgetId::named("Lorem 2"), 200.pct()),
                    scroll_to_zoom_btn(WidgetId::named("Lorem 2"), 50.pct()),
                ]
            ),
            SubMenu! {
                "Options",
                ui_vec![
                    Toggle! {
                        checked = mouse_pan;
                        child = Text!("Mouse Pan");
                    },
                    Toggle! {
                        checked = smooth_scrolling;
                        child = Text!("Smooth Scrolling");
                    },
                ]
            }
        ];
    }
}
fn scroll_to_btn(target: WidgetId, mode: ScrollToMode) -> impl UiNode {
    use zero_ui::scroll::cmd;

    let scroll = WidgetId::named("scroll");
    let cmd = cmd::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} {}", target, if let ScrollToMode::Minimal {..} = &mode { "(minimal)" } else { "(center)" });
        cmd_param = CommandParam::new(cmd::ScrollToRequest { target: target.into(), mode: mode.clone(), zoom: None, });
        cmd;
    }
}
fn scroll_to_zoom_btn(target: WidgetId, zoom: layout::FactorPercent) -> impl UiNode {
    use zero_ui::scroll::cmd;

    let scroll = WidgetId::named("scroll");
    let cmd = cmd::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} (minimal) at {}", target, zoom);
        cmd_param = CommandParam::new(cmd::ScrollToRequest { target: target.into(), mode: ScrollToMode::minimal(10), zoom: Some(zoom.into()), });
        cmd;
    }
}

fn scroll_to_rect(target: layout::Rect, mode: ScrollToMode) -> impl UiNode {
    use zero_ui::scroll::cmd;

    let scroll = WidgetId::named("scroll");
    let cmd = cmd::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} {}", target, if let ScrollToMode::Minimal {..} = &mode { "(minimal)" } else { "(center)" });
        cmd_param = CommandParam::new(cmd::ScrollToRequest { target: target.clone().into(), mode: mode.clone(), zoom: None, });
        cmd;
    }
}

fn ipsum() -> Txt {
    let mut s = String::new();
    let mut rng = rand::rngs::StdRng::from_seed([0; 32]);
    for _ in 0..10 {
        for _ in 0..10 {
            s.push_str(&lipsum::lipsum_words_with_rng(&mut rng, 25));
            s.push('\n');
        }
        s.push('\n');
    }

    s.into()
}
