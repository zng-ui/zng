//! Demonstrates the `Scroll!` widget and scroll commands.

use zng::{
    mouse::{CursorIcon, cursor},
    prelude::*,
    scroll::cmd::ScrollToMode,
};

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        let mouse_pan = var(false);
        let smooth_scrolling = var(true);
        Window! {
            title = "Scroll Example";
            child_top = commands(mouse_pan.clone(), smooth_scrolling.clone()), 0;
            child = Scroll! {
                id = "scroll";
                padding = 20;
                widget::background_color = light_dark(
                    colors::WHITE.with_alpha(80.pct()).mix_normal(hex!(#245E81)),
                    hex!(#245E81),
                );
                smooth_scrolling = smooth_scrolling.map_into();

                mouse_pan;
                when *#mouse_pan {
                    cursor = CursorIcon::Grab;
                }
                when *#mouse_pan && *#gesture::is_pressed {
                    cursor = CursorIcon::Grabbing;
                }

                child = Stack! {
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
                };
            };
        }
    })
}

fn commands(mouse_pan: Var<bool>, smooth_scrolling: Var<bool>) -> UiNode {
    use zng::scroll::cmd::*;

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
                    Button!(ZOOM_TO_FIT_CMD.scoped(scope)),
                    Button!(ZOOM_RESET_CMD.scoped(scope)),
                    Hr!(),
                    scroll_to_zoom_btn(WidgetId::named("Lorem 2"), 200.pct()),
                    scroll_to_zoom_btn(WidgetId::named("Lorem 2"), 50.pct()),
                ]
            ),
            SubMenu!(
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
            )
        ];
    }
}
fn scroll_to_btn(target: WidgetId, mode: ScrollToMode) -> UiNode {
    use zng::scroll::cmd;

    let scroll = WidgetId::named("scroll");
    let cmd = cmd::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!(
            "Scroll To {} {}",
            target,
            if let ScrollToMode::Minimal { .. } = &mode {
                "(minimal)"
            } else {
                "(center)"
            }
        );
        cmd_param = cmd::ScrollToRequest {
            target: target.into(),
            mode: mode.clone(),
            zoom: None,
        };
        cmd;
    }
}
fn scroll_to_zoom_btn(target: WidgetId, zoom: layout::FactorPercent) -> UiNode {
    use zng::scroll::cmd;

    let scroll = WidgetId::named("scroll");
    let cmd = cmd::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!("Scroll To {} (minimal) at {}", target, zoom);
        cmd_param = cmd::ScrollToRequest {
            target: target.into(),
            mode: ScrollToMode::minimal(10),
            zoom: Some(zoom.into()),
        };
        cmd;
    }
}

fn scroll_to_rect(target: layout::Rect, mode: ScrollToMode) -> UiNode {
    use zng::scroll::cmd;

    let scroll = WidgetId::named("scroll");
    let cmd = cmd::SCROLL_TO_CMD.scoped(scroll);
    Button! {
        child = Text!(
            "Scroll To {} {}",
            target,
            if let ScrollToMode::Minimal { .. } = &mode {
                "(minimal)"
            } else {
                "(center)"
            }
        );
        cmd_param = cmd::ScrollToRequest {
            target: target.clone().into(),
            mode: mode.clone(),
            zoom: None,
        };
        cmd;
    }
}

fn ipsum() -> Txt {
    static IPSUM: &[&str] = &[
        "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam interdum tincidunt nibh, id placerat purus molestie at. Quisque fringilla viverra quam at pretium. Cras volutpat vehicula mauris, sit amet efficitur tortor tempor eu. Aenean fermentum condimentum nunc, sed fringilla justo aliquam a. Curabitur porta iaculis venenatis. Nunc dignissim suscipit nunc, vel lobortis purus tincidunt eu. Sed condimentum sollicitudin ligula. Sed a pulvinar felis, nec mattis metus. Proin rhoncus risus id enim consequat hendrerit. Nulla pharetra ante sem, id aliquam quam semper nec. Suspendisse efficitur lacinia ligula, in vehicula nunc luctus nec. Proin condimentum tortor vel ex bibendum molestie. Aenean quis lobortis orci. ",
        "Sed et dignissim neque, sed scelerisque lectus. Integer sit amet nisi dui. Aliquam malesuada arcu quis nunc finibus auctor. Fusce auctor est a est lobortis, in tempor ligula eleifend. Ut pulvinar magna nec nibh efficitur dapibus. Praesent pretium eleifend lacinia. Etiam sed elementum est. Nulla ullamcorper mauris at ullamcorper aliquam. Mauris nibh sem, convallis sed facilisis eget, viverra ut orci. Ut elementum erat eget congue malesuada. Maecenas ut elementum nisl. Aenean ut magna sapien. Praesent iaculis ante sit amet leo placerat, vitae tempus purus egestas. Phasellus tincidunt, purus eget tempus tristique, ligula ex euismod elit, non facilisis libero lectus id orci. ",
        "Etiam in pulvinar metus, ac gravida justo. Vestibulum suscipit suscipit ligula, a faucibus lectus rhoncus ut. Aliquam quis ipsum vel enim fringilla facilisis. Cras a augue nibh. Nulla purus lorem, accumsan nec mi a, gravida aliquam metus. Vestibulum mollis imperdiet pharetra. Vestibulum tempor rutrum molestie. Phasellus nec porta mauris. Pellentesque sagittis est sem, vitae commodo libero viverra a. In hac habitasse platea dictumst. Aenean vitae dui eu dui posuere sagittis eget id orci. Praesent purus elit, imperdiet quis felis et, placerat eleifend sapien. Curabitur diam diam, convallis sed mi eu, maximus sagittis nisi. Vivamus mauris sem, condimentum quis ultrices eget, porta eu elit. Aliquam lorem arcu, ultricies nec lorem ut, mattis vulputate erat. Nullam lacinia magna nec consequat egestas. ",
        "Quisque ornare erat vel turpis tempus cursus. In bibendum bibendum lectus eu condimentum. Pellentesque nec orci metus. Maecenas ac odio quis odio auctor tempus. Vivamus tristique tempor nisi. Donec ante augue, tempus vel tincidunt luctus, pulvinar ac felis. Proin magna eros, finibus ut pulvinar imperdiet, elementum vel ligula. Cras eu vestibulum orci. Proin et quam et eros interdum imperdiet. Nulla facilisi. Proin convallis luctus risus et suscipit. Donec molestie id augue eu semper. Nulla venenatis nisl risus, non aliquam orci eleifend sed. Etiam sodales porta nisl, posuere vestibulum massa gravida a. Praesent sit amet hendrerit ipsum. Nunc nec purus consectetur, consectetur ex vel, egestas justo. ",
    ];
    let mut i = 0;
    let mut r = String::new();
    for _ in 0..30 {
        let mut line_len = 0;
        for word in IPSUM[i].split(' ') {
            line_len += word.len();
            if line_len > 150 {
                line_len = word.len();
                r.push('\n');
            }
            r.push_str(word);
            r.push(' ');
        }
        r.push('\n');
        r.push('\n');

        i += 1;
        if i == IPSUM.len() {
            i = 0;
        }
    }
    r.into()
}
