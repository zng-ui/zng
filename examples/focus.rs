#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::focus::FocusChangedEvent;
use zero_ui::prelude::*;
use zero_ui::widgets::window::{LayerIndex, WindowLayers};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // zero_ui_view::run_same_process(app_main);
    app_main();
}

fn app_main() {
    App::default().run_window(|ctx| {
        trace_focus(ctx.events);
        let window_enabled = var(true);
        window! {
            title = "Focus Example";
            enabled = window_enabled.clone();
            content_align = unset!;
            content = v_stack! {
                items = widgets![
                    alt_scope(),
                    h_stack! {
                        margin = (50, 0, 0, 0);
                        align = Alignment::CENTER;
                        spacing = 10;
                        items = widgets![
                            tab_index(),
                            functions(window_enabled)
                        ]
                    }
                ];
            };
            // zero_ui::widgets::inspector::show_center_points = true;
            // zero_ui::widgets::inspector::show_bounds = true;
        }
    })
}

fn alt_scope() -> impl Widget {
    h_stack! {
        alt_focus_scope = true;
        button::theme::border = 0, BorderStyle::Solid;
        button::theme::corner_radius = 0;
        items = widgets![
            button("alt", TabIndex::AUTO),
            button("scope", TabIndex::AUTO),
        ];
    }
}

fn tab_index() -> impl Widget {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(T);
        items = widgets![
            title("TabIndex (T)"),
            button("Button A5", 5),
            button("Button A4", 3),
            button("Button A3", 2),
            button("Button A1", 0),
            button("Button A2", 0),
        ];
    }
}

fn functions(window_enabled: RcVar<bool>) -> impl Widget {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(F);
        items = widgets![
            title("Functions (F)"),
            // New Window
            button! {
                content = text("New Window");
                on_click = hn!(|ctx, _| {
                    ctx.services.windows().open(|_|window! {
                        title = "Other Window";
                        focus_shortcut = shortcut!(W);
                        content = v_stack! {
                            spacing = 5;
                            items = widgets![
                                title("Other Window (W)"),
                                button("Button B5", 5),
                                button("Button B4", 3),
                                button("Button B3", 2),
                                button("Button B1", 0),
                                button("Button B2", 0),
                            ]
                        };
                    });
                });
            },
            // Detach Button
            {
                let detach_focused = RcNode::new_cyclic(|wk| {
                    let btn = button! {
                        content = text("Detach Button");
                        on_click = hn!(|ctx, _| {
                            let wwk = wk.clone();
                            ctx.services.windows().open(move |_| {
                                window! {
                                    title = "Detached Button";
                                    content = slot(wwk.upgrade().unwrap(), take_on_init());
                                }
                            });
                        });
                    };
                    btn.boxed()
                });
                slot(detach_focused, take_on_init())
            },
            // Disable Window
            disable_window(window_enabled.clone()),
            // Overlay Scope
            button! {
                content = text("Overlay Scope");
                on_click = hn!(|ctx, _| {
                    WindowLayers::insert(ctx, LayerIndex::TOP_MOST, overlay(window_enabled.clone()));
                });
            }
        ]
    }
}
fn disable_window(window_enabled: RcVar<bool>) -> impl Widget {
    button! {
        content = text(window_enabled.map(|&e| if e { "Disable Window" } else { "Enabling in 1s..." }.into()));
        width = 140;
        on_click = async_hn!(window_enabled, |ctx, _| {
            window_enabled.set(&ctx, false);
            task::timeout(1.secs()).await;
            window_enabled.set(&ctx, true);
        });
    }
}
fn overlay(window_enabled: RcVar<bool>) -> impl Widget {
    container! {
        id = "overlay";
        modal = true;
        content = container! {
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            background_color = rgb(0.05, 0.05, 0.05);
            drop_shadow = (0, 0), 4, colors::BLACK;
            padding = 2;
            content = v_stack! {
                items_align = Alignment::RIGHT;
                items = widgets![
                    text! {
                        text = "Window scope is disabled so the overlay scope is the root scope.";
                        margin = 15;
                    },
                    h_stack! {
                        spacing = 2;
                        items = widgets![
                        disable_window(window_enabled),
                        button! {
                                content = text("Close");
                                on_click = hn!(|ctx, _| {
                                    WindowLayers::remove(ctx, "overlay");
                                })
                            }
                        ]
                    }
                ]
            }
        }
    }
}

fn title(text: impl IntoVar<Text>) -> impl Widget {
    text! { text; font_weight = FontWeight::BOLD; align = Alignment::CENTER; }
}

fn button(content: impl Into<Text>, tab_index: impl Into<TabIndex>) -> impl Widget {
    let content = content.into();
    let tab_index = tab_index.into();
    button! {
        content = text(content.clone());
        tab_index;
        on_click = hn!(|_, _| {
            println!("Clicked {content} {tab_index:?}")
        });
    }
}

fn trace_focus(events: &mut Events) {
    events
        .on_pre_event(
            FocusChangedEvent,
            app_hn!(|ctx, args: &FocusChangedArgs, _| {
                if args.is_hightlight_changed() {
                    println!("highlight: {}", args.highlight);
                } else if args.is_widget_move() {
                    println!("focused {:?} moved", args.new_focus.as_ref().unwrap());
                } else {
                    println!(
                        "{} -> {}",
                        inspect::focus(&args.prev_focus, ctx.services),
                        inspect::focus(&args.new_focus, ctx.services)
                    );
                }
            }),
        )
        .permanent();
}

#[cfg(debug_assertions)]
mod inspect {
    use super::*;
    use zero_ui::core::focus::WidgetInfoFocusExt;
    use zero_ui::core::inspector::{WidgetInspectorInfo, WidgetNewFn};

    pub fn focus(path: &Option<WidgetPath>, services: &mut Services) -> String {
        path.as_ref()
            .map(|p| {
                let frame = if let Ok(w) = services.windows().widget_tree(p.window_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let widget = if let Some(w) = frame.get(p) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let info = widget.instance().expect("expected debug info").borrow();

                if info.widget_name == "button" {
                    let text_wgt = widget.descendants().next().expect("expected text in button");
                    let info = text_wgt.instance().expect("expected debug info").borrow();
                    format!(
                        "button({})",
                        info.captures
                            .get(&WidgetNewFn::NewChild)
                            .unwrap()
                            .iter()
                            .find(|p| p.property_name == "text")
                            .expect("expected text in capture_new")
                            .args[0]
                            .value
                            .debug
                    )
                } else if info.widget_name == "window" {
                    let title = widget
                        .properties()
                        .iter()
                        .find(|p| p.borrow().property_name == "title")
                        .map(|p| p.borrow().args[0].value.debug.clone())
                        .unwrap_or_default();
                    format!("window({title})")
                } else {
                    let focus_info = widget.as_focus_info();
                    if focus_info.is_alt_scope() {
                        format!("{}(is_alt_scope)", info.widget_name)
                    } else if focus_info.is_scope() {
                        format!("{}(is_scope)", info.widget_name)
                    } else {
                        info.widget_name.to_owned()
                    }
                }
            })
            .unwrap_or_else(|| "<none>".to_owned())
    }
}

#[cfg(not(debug_assertions))]
mod inspect {
    use super::*;

    pub fn focus(path: &Option<WidgetPath>, _: &mut Services) -> String {
        path.as_ref()
            .map(|p| format!("{:?}", p.widget_id()))
            .unwrap_or_else(|| "<none>".to_owned())
    }
}
