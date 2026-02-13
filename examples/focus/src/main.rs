//! Demonstrates the focus service, logical and directional navigation.

use zng::{
    button,
    color::filter::drop_shadow,
    focus::{
        DirectionalNav, FOCUS_CHANGED_EVENT, FocusClickBehavior, FocusRequest, TabIndex, TabNav, alt_focus_scope, directional_nav,
        focus_click_behavior, focus_scope, focus_shortcut, focusable, is_focused, is_return_focus, return_focus_on_deinit, tab_index,
        tab_nav,
    },
    font::FontName,
    layout::{align, margin, padding},
    prelude::*,
    text::font_color,
    widget::{background_color, border, corner_radius, enabled, node::ArcNode},
    window::FocusIndicator,
};

fn main() {
    zng::env::init!();
    // zng::app::print_tracing(tracing::Level::TRACE, false, |m| m.target().contains("zng_ext_input"));

    APP.defaults().run_window("main", async {
        WINDOW.id().set_name("main").unwrap();

        trace_focus();
        let window_enabled = var(true);
        Window! {
            title = "Focus Example";
            enabled = window_enabled.clone();
            child_top = alt_scope();
            child_spacing = 50;
            child = Stack! {
                direction = StackDirection::left_to_right();
                align = Align::TOP;
                spacing = 10;
                children = ui_vec![tab_index_example(), functions(window_enabled), delayed_focus()];
            };
            widget::background = commands();
            // zng::window::inspector::show_center_points = true;
            // zng::window::inspector::show_bounds = true;
            // zng::window::inspector::show_hit_test = true;
            // zng::window::inspector::show_directional_query = Some(layout::Orientation2D::Below);
        }
    })
}

fn alt_scope() -> UiNode {
    Stack! {
        direction = StackDirection::left_to_right();
        alt_focus_scope = true;
        focus_click_behavior = FocusClickBehavior::menu_item();
        button::style_fn = Style! {
            border = unset!;
            corner_radius = unset!;
        };
        children = ui_vec![button("alt", TabIndex::AUTO), button("scope", TabIndex::AUTO),];
    }
}

fn tab_index_example() -> UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        focus_shortcut = shortcut!('T');
        children = ui_vec![
            title("TabIndex (T)"),
            button("Button A5", 5),
            button("Button A4", 3),
            button("Button A3", 2),
            button("Button A1", 0),
            button("Button A2", 0),
        ];
    }
}

fn functions(window_enabled: Var<bool>) -> UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        focus_shortcut = shortcut!('F');
        children = ui_vec![
            title("Functions (F)"),
            // New Window
            Button! {
                child = Text!("New Window");
                on_click = hn!(|_| {
                    WINDOWS.open(WindowId::new_unique(), async {
                        let _ = WINDOW.id().set_name("other"); // name first other
                        Window! {
                            title = "Other Window";
                            focus_shortcut = shortcut!('W');
                            child = Stack! {
                                direction = StackDirection::top_to_bottom();
                                align = Align::CENTER;
                                spacing = 5;
                                children = ui_vec![
                                    title("Other Window (W)"),
                                    button("Button B5", 5),
                                    button("Button B4", 3),
                                    button("Button B3", 2),
                                    button("Button B1", 0),
                                    button("Button B2", 0),
                                ];
                            };
                        }
                    });
                });
            },
            // Detach Button
            {
                let detach_focused = ArcNode::new_cyclic(|wk| {
                    Button! {
                        id = "detachable-btn";
                        child = Text!("Detach Button");
                        // focus_on_init = true;
                        on_click = hn!(|_| {
                            let wwk = wk.clone();
                            WINDOWS.open(WindowId::new_unique(), async move {
                                Window! {
                                    title = "Detached Button";
                                    child_align = Align::CENTER;
                                    child = wwk.upgrade().unwrap().take_on_init();
                                    // child_bottom = Button! {
                                    //     child = Text!("Test");
                                    // };
                                }
                            });
                        });
                    }
                });
                detach_focused.take_on_init().into_widget()
            },
            // Disable Window
            disable_window(window_enabled.clone()),
            // Overlay Scope
            Button! {
                id = "overlay-scope-btn";
                child = Text!("Overlay Scope");
                on_click = hn!(|_| {
                    LAYERS.insert(LayerIndex::TOP_MOST, overlay(window_enabled.clone()));
                });
            },
            nested_focusables(),
        ];
    }
}
fn disable_window(window_enabled: Var<bool>) -> UiNode {
    Button! {
        child = Text!(
            window_enabled.map(|&e| if e { "Disable Window" } else { "Enabling in 1s..." }.into())
        );
        layout::min_width = 140;
        on_click = async_hn!(window_enabled, |_| {
            window_enabled.set(false);
            task::deadline(1.secs()).await;
            window_enabled.set(true);
        });
    }
}
fn overlay(window_enabled: Var<bool>) -> UiNode {
    Container! {
        id = "overlay";
        widget::modal = true;
        return_focus_on_deinit = true;
        child_align = Align::CENTER;
        child = Container! {
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            background_color = light_dark(colors::WHITE.with_alpha(90.pct()), colors::BLACK.with_alpha(90.pct()));
            drop_shadow = (0, 0), 4, colors::BLACK;
            padding = 2;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                children_align = Align::RIGHT;
                children = ui_vec![
                    Text! {
                        txt = "Window scope is disabled so the overlay scope is the root scope.";
                        margin = 15;
                    },
                    Stack! {
                        direction = StackDirection::left_to_right();
                        spacing = 2;
                        children = ui_vec![
                            disable_window(window_enabled),
                            Button! {
                                child = Text!("Close");
                                on_click = hn!(|_| {
                                    LAYERS.remove("overlay");
                                });
                            }
                        ];
                    }
                ];
            };
        };
    }
}

fn delayed_focus() -> UiNode {
    Stack! {
        direction = StackDirection::top_to_bottom();
        spacing = 5;
        focus_shortcut = shortcut!('D');
        children = ui_vec![
            title("Delayed 4s (D)"),
            delayed_btn("Force Focus", || {
                FOCUS.focus({
                    let mut req = FocusRequest::direct("target".into(), true);
                    req.force_window_focus = true;
                    req
                });
            }),
            delayed_btn("Info Indicator", || {
                FOCUS.focus({
                    let mut req = FocusRequest::direct("target".into(), true);
                    req.window_indicator = Some(FocusIndicator::Info);
                    req
                });
            }),
            delayed_btn("Critical Indicator", || {
                FOCUS.focus({
                    let mut req = FocusRequest::direct("target".into(), true);
                    req.window_indicator = Some(FocusIndicator::Critical);
                    req
                });
            }),
            Text! {
                id = "target";
                padding = (7, 15);
                corner_radius = 4;
                txt = "delayed target";
                font_style = FontStyle::Italic;
                txt_align = Align::CENTER;
                background_color = light_dark(rgb(225, 225, 225), rgb(30, 30, 30));

                focusable = true;
                when *#is_focused {
                    txt = "focused";
                    background_color = light_dark(web_colors::LIGHT_GREEN, web_colors::DARK_GREEN);
                }
            },
        ];
    }
}
fn delayed_btn(content: impl Into<Txt>, on_timeout: impl FnMut() + Send + 'static) -> UiNode {
    use std::sync::Arc;
    use task::parking_lot::Mutex;

    let on_timeout = Arc::new(Mutex::new(Box::new(on_timeout)));
    let enabled = var(true);
    Button! {
        child = Text!(content.into());
        on_click = async_hn!(enabled, on_timeout, |_| {
            enabled.set(false);
            task::deadline(4.secs()).await;
            let mut on_timeout = on_timeout.lock();
            on_timeout();
            enabled.set(true);
        });
        enabled;
    }
}

fn title(txt: impl IntoVar<Txt>) -> UiNode {
    Text! {
        txt;
        font_weight = FontWeight::BOLD;
        align = Align::CENTER;
    }
}

fn button(content: impl Into<Txt>, tab_index: impl Into<TabIndex>) -> UiNode {
    let content = content.into();
    let tab_index = tab_index.into();
    Button! {
        child = Text!(content.clone());
        tab_index;
        on_click = hn!(|_| tracing::info!("Clicked {content} {tab_index:?}"));
    }
}

fn commands() -> UiNode {
    use zng::focus::cmd::*;

    let cmds = [
        FOCUS_NEXT_CMD,
        FOCUS_PREV_CMD,
        FOCUS_UP_CMD,
        FOCUS_RIGHT_CMD,
        FOCUS_DOWN_CMD,
        FOCUS_LEFT_CMD,
        FOCUS_ALT_CMD,
        FOCUS_ENTER_CMD,
        FOCUS_EXIT_CMD,
    ];

    Stack! {
        direction = StackDirection::top_to_bottom();
        align = Align::BOTTOM_RIGHT;
        padding = 10;
        spacing = 8;
        children_align = Align::RIGHT;

        text::font_family = FontName::monospace();
        font_color = colors::GRAY;

        children = cmds.into_iter().map(|cmd| {
            Text! {
                txt = cmd.name_with_shortcut();

                when *#{cmd.is_enabled()} {
                    font_color = light_dark(colors::BLACK, colors::WHITE);
                }
            }
        });
    }
}

fn trace_focus() {
    FOCUS_CHANGED_EVENT
        .on_pre_event(
            true,
            hn!(|args| {
                if args.is_highlight_changed() {
                    tracing::info!("highlight: {}", args.highlight);
                } else if args.is_widget_move() {
                    tracing::info!("focused {:?} moved", args.new_focus.as_ref().unwrap());
                } else if args.is_enabled_change() {
                    tracing::info!("focused {:?} enabled/disabled", args.new_focus.as_ref().unwrap());
                } else {
                    tracing::info!("{} -> {}", inspect::focus(&args.prev_focus), inspect::focus(&args.new_focus));
                }
            }),
        )
        .perm();
}

fn nested_focusables() -> UiNode {
    Button! {
        child = Text!("Nested Focusables");

        on_click = hn!(|args| {
            args.propagation.stop();
            WINDOWS.focus_or_open("nested-focusables", async {
                Window! {
                    title = "Focus Example - Nested Focusables";

                    color_scheme = color::ColorScheme::Dark;
                    background_color = web_colors::DIM_GRAY;

                    // zng::properties::inspector::show_center_points = true;
                    child_align = Align::CENTER;
                    child = Stack! {
                        direction = StackDirection::top_to_bottom();
                        spacing = 10;
                        children = ui_vec![nested_focusables_group('a'), nested_focusables_group('b')];
                    };
                }
            });
        });
    }
}
fn nested_focusables_group(g: char) -> UiNode {
    Stack! {
        direction = StackDirection::left_to_right();
        align = Align::TOP;
        spacing = 10;
        children = (0..5).map(|n| nested_focusable(g, n, 0));
    }
}
fn nested_focusable(g: char, column: u8, row: u8) -> UiNode {
    let nested = Text! {
        txt = format!("Focusable {column}, {row}");
        margin = 5;
    };
    Stack! {
        id = formatx!("focusable-{g}-{column}-{row}");
        padding = 2;
        direction = StackDirection::top_to_bottom();
        children = if row == 5 {
            ui_vec![nested]
        } else {
            ui_vec![nested, nested_focusable(g, column, row + 1),]
        };

        focusable = true;

        corner_radius = 5;
        border = 1, colors::RED.with_alpha(30.pct());
        background_color = colors::RED.with_alpha(20.pct());
        when *#is_focused {
            background_color = web_colors::GREEN;
        }
        when *#is_return_focus {
            border = 1, web_colors::LIME_GREEN;
        }
    }
}

#[cfg(debug_assertions)]
mod inspect {
    use super::*;

    pub fn focus(path: &Option<widget::info::InteractionPath>) -> String {
        path.as_ref()
            .map(|p| {
                let frame = if let Some(w) = WINDOWS.widget_tree(p.window_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let widget = if let Some(w) = frame.get(p.widget_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let wgt_type = if let Some(b) = widget.inspector_info() {
                    b.builder.widget_type()
                } else {
                    return format!("<{p}>");
                };
                if wgt_type.path.ends_with("Button") {
                    let txt = widget
                        .inspect_descendant("Text")
                        .expect("expected text in button")
                        .inspect_property("txt")
                        .expect("expected txt property in text")
                        .live_debug(0)
                        .get();

                    format!("Button({txt})")
                } else if wgt_type.path.ends_with("Window") {
                    let title = widget.inspect_property("title").map(|p| p.live_debug(0).get()).unwrap_or_default();

                    format!("Window({title})")
                } else {
                    let focus_info = widget.into_focus_info(true, true);
                    if focus_info.is_alt_scope() {
                        format!("{}(is_alt_scope)", wgt_type.name())
                    } else if focus_info.is_scope() {
                        format!("{}(is_scope)", wgt_type.name())
                    } else {
                        format!("{}({})", wgt_type.name(), p.widget_id())
                    }
                }
            })
            .unwrap_or_else(|| "<none>".to_owned())
    }
}

#[cfg(not(debug_assertions))]
mod inspect {
    pub fn focus(path: &Option<zng::widget::info::InteractionPath>) -> String {
        path.as_ref()
            .map(|p| format!("{:?}", p.widget_id()))
            .unwrap_or_else(|| "<none>".to_owned())
    }
}
