#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use zero_ui::{
    gesture::is_pressed,
    icon::{self, Icon},
    mouse::{cursor, CursorIcon},
    prelude::*,
    scroll::commands::ScrollToMode,
    widget::background_color,
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
            child_insert_above = commands(mouse_pan.clone(), smooth_scrolling.clone()), 0;
            child = Scroll! {
                id = "scroll";
                padding = 20;
                background_color = color_scheme_map(
                    hex!(#245E81),
                    colors::WHITE.with_alpha(80.pct()).mix_normal(hex!(#245E81))
                );
                smooth_scrolling = smooth_scrolling.map_into();

                mouse_pan;
                when *#mouse_pan {
                    cursor = CursorIcon::Grab;
                }
                when *#mouse_pan && *#is_pressed {
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
    use zero_ui::scroll::commands::*;

    SCROLL_TO_TOP_CMD.init_icon(wgt_fn!(|_| Icon!(icon::outlined::VERTICAL_ALIGN_TOP)));
    SCROLL_TO_BOTTOM_CMD.init_icon(wgt_fn!(|_| Icon!(icon::outlined::VERTICAL_ALIGN_BOTTOM)));

    let scope = WidgetId::named("scroll");
    use menu::CmdButton;

    Menu! {
        id = "menu";
        children = ui_vec![
            SubMenu!(
                "Scroll",
                ui_vec![
                    CmdButton!(SCROLL_UP_CMD.scoped(scope)),
                    CmdButton!(SCROLL_DOWN_CMD.scoped(scope)),
                    CmdButton!(SCROLL_LEFT_CMD.scoped(scope)),
                    CmdButton!(SCROLL_RIGHT_CMD.scoped(scope)),
                ]
            ),
            SubMenu!(
                "Page",
                ui_vec![
                    CmdButton!(PAGE_UP_CMD.scoped(scope)),
                    CmdButton!(PAGE_DOWN_CMD.scoped(scope)),
                    CmdButton!(PAGE_LEFT_CMD.scoped(scope)),
                    CmdButton!(PAGE_RIGHT_CMD.scoped(scope)),
                ]
            ),
            SubMenu!(
                "Scroll to",
                ui_vec![
                    CmdButton!(SCROLL_TO_TOP_CMD.scoped(scope)),
                    CmdButton!(SCROLL_TO_BOTTOM_CMD.scoped(scope)),
                    CmdButton!(SCROLL_TO_LEFTMOST_CMD.scoped(scope)),
                    CmdButton!(SCROLL_TO_RIGHTMOST_CMD.scoped(scope)),
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
                    CmdButton!(ZOOM_IN_CMD.scoped(scope)),
                    CmdButton!(ZOOM_OUT_CMD.scoped(scope)),
                    CmdButton!(ZOOM_RESET_CMD.scoped(scope)),
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
    use zero_ui::scroll::commands;

    let scroll = WidgetId::named("scroll");
    let cmd = commands::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} {}", target, if let ScrollToMode::Minimal {..} = &mode { "(minimal)" } else { "(center)" });
        enabled = cmd.is_enabled();
        on_click = hn!(|_| {
            cmd.notify_param(commands::ScrollToRequest { target: target.into(), mode: mode.clone(), zoom: None, });
        });
    }
}
fn scroll_to_zoom_btn(target: WidgetId, zoom: FactorPercent) -> impl UiNode {
    use zero_ui::scroll::commands;

    let scroll = WidgetId::named("scroll");
    let cmd = commands::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} (minimal) at {}", target, zoom);
        enabled = cmd.is_enabled();
        on_click = hn!(|_| {
            cmd.notify_param(commands::ScrollToRequest { target: target.into(), mode: ScrollToMode::minimal(10), zoom: Some(zoom.into()), });
        });
    }
}

fn scroll_to_rect(target: Rect, mode: ScrollToMode) -> impl UiNode {
    use zero_ui::scroll::commands;

    let scroll = WidgetId::named("scroll");
    let cmd = commands::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} {}", target, if let ScrollToMode::Minimal {..} = &mode { "(minimal)" } else { "(center)" });
        enabled = cmd.is_enabled();
        on_click = hn!(|_| {
            cmd.notify_param(commands::ScrollToRequest { target: target.clone().into(), mode: mode.clone(), zoom: None, });
        });
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
