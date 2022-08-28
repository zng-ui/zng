#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::core::focus::{FocusChangedEvent, FocusRequest, FocusTarget};
use zero_ui::prelude::*;
use zero_ui::widgets::window::{LayerIndex, WindowLayers};

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    // let rec = examples_util::record_profile("focus");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    // rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        ctx.window_id.set_name("main").unwrap();

        trace_focus(ctx.events);
        let window_enabled = var(true);
        window! {
            title = "Focus Example";
            enabled = window_enabled.clone();
            background = commands();
            content = v_stack! {
                items = widgets![
                    alt_scope(),
                    h_stack! {
                        margin = (50, 0, 0, 0);
                        align = Align::CENTER;
                        spacing = 10;
                        items = widgets![
                            tab_index(),
                            functions(window_enabled),
                            delayed_focus(),
                        ]
                    }
                ];
            };
            // zero_ui::properties::inspector::show_center_points = true;
            // zero_ui::properties::inspector::show_bounds = true;
            // zero_ui::properties::inspector::show_hit_test = true;
            // zero_ui::properties::inspector::show_directional_query = Some(zero_ui::core::units::Orientation2D::Below);
        }
    })
}

fn alt_scope() -> impl Widget {
    h_stack! {
        alt_focus_scope = true;
        button::vis::dark = theme_generator!(|_, _| {
            button::vis::dark_theme! {
                border = unset!;
                corner_radius = unset!;
            }
        });
        button::vis::light = theme_generator!(|_, _| {
            button::vis::light_theme! {
                border = unset!;
                corner_radius = unset!;
            }
        });
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
                    Windows::req(ctx.services).open(|ctx| {
                        let _ = ctx.window_id.set_name("other");
                        window! {
                            title = "Other Window";
                            focus_shortcut = shortcut!(W);
                            content = v_stack! {
                                align = Align::CENTER;
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
                        }
                    });
                });
            },
            // Detach Button
            {
                let detach_focused = RcNode::new_cyclic(|wk| {
                    let btn = button! {
                        content = text("Detach Button");
                        // focus_on_init = true;
                        on_click = hn!(|ctx, _| {
                            let wwk = wk.clone();
                            Windows::req(ctx.services).open(move |_| {
                                window! {
                                    title = "Detached Button";
                                    content_align = Align::CENTER;
                                    content = slot(wwk.upgrade().unwrap(), slot::take_on_init());
                                }
                            });
                        });
                    };
                    btn.boxed()
                });
                slot(detach_focused, slot::take_on_init())
            },
            // Disable Window
            disable_window(window_enabled.clone()),
            // Overlay Scope
            button! {
                content = text("Overlay Scope");
                on_click = hn!(|ctx, _| {
                    WindowLayers::insert(ctx, LayerIndex::TOP_MOST, overlay(window_enabled.clone()));
                });
            },
            nested_focusables(),
        ]
    }
}
fn disable_window(window_enabled: RcVar<bool>) -> impl Widget {
    button! {
        content = text(window_enabled.map(|&e| if e { "Disable Window" } else { "Enabling in 1s..." }.into()));
        min_width = 140;
        on_click = async_hn!(window_enabled, |ctx, _| {
            window_enabled.set(&ctx, false);
            task::deadline(1.secs()).await;
            window_enabled.set(&ctx, true);
        });
    }
}
fn overlay(window_enabled: RcVar<bool>) -> impl Widget {
    container! {
        id = "overlay";
        modal = true;
        content_align = Align::CENTER;
        content = container! {
            focus_scope = true;
            tab_nav = TabNav::Cycle;
            directional_nav = DirectionalNav::Cycle;
            background_color = rgb(0.05, 0.05, 0.05);
            drop_shadow = (0, 0), 4, colors::BLACK;
            padding = 2;
            content = v_stack! {
                items_align = Align::RIGHT;
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

fn delayed_focus() -> impl Widget {
    v_stack! {
        spacing = 5;
        focus_shortcut = shortcut!(D);
        items = widgets![
            title("Delayed 4s (D)"),

            delayed_btn("Force Focus", |ctx| {
                Focus::req(ctx.services).focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: true,
                    window_indicator: None,
                });
            }),
            delayed_btn("Info Indicator", |ctx| {
                Focus::req(ctx.services).focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: false,
                    window_indicator: Some(FocusIndicator::Info),
                });
            }),
            delayed_btn("Critical Indicator", |ctx| {
                Focus::req(ctx.services).focus(FocusRequest {
                    target: FocusTarget::Direct(WidgetId::named("target")),
                    highlight: true,
                    force_window_focus: false,
                    window_indicator: Some(FocusIndicator::Critical),
                });
            }),

            text! {
                id = "target";
                padding = (7, 15);
                corner_radius = 4;
                text = "delayed target";
                font_style = FontStyle::Italic;
                text_align = TextAlign::CENTER_MIDDLE;
                background_color = rgb(30, 30, 30);

                focusable = true;
                when self.is_focused {
                    text = "focused";
                    background_color = colors::DARK_GREEN;
                }
            },
        ]
    }
}
fn delayed_btn(content: impl Into<Text>, on_timeout: impl FnMut(&mut WidgetContext) + 'static) -> impl Widget {
    let on_timeout = std::rc::Rc::new(std::cell::RefCell::new(Box::new(on_timeout)));
    let enabled = var(true);
    button! {
        content = text(content.into());
        on_click = async_hn!(enabled, on_timeout, |ctx, _| {
            enabled.set(&ctx, false);
            task::deadline(4.secs()).await;
            ctx.with(|ctx| {
                let mut on_timeout = on_timeout.borrow_mut();
                on_timeout(ctx);
            });
            enabled.set(&ctx, true);
        });
        enabled;
    }
}

fn title(text: impl IntoVar<Text>) -> impl Widget {
    text! { text; font_weight = FontWeight::BOLD; align = Align::CENTER; }
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

fn commands() -> impl Widget {
    use zero_ui::core::focus::commands::*;

    let cmds = [
        FocusNextCommand.as_any(),
        FocusPrevCommand.as_any(),
        FocusUpCommand.as_any(),
        FocusRightCommand.as_any(),
        FocusDownCommand.as_any(),
        FocusLeftCommand.as_any(),
        FocusAltCommand.as_any(),
        FocusEnterCommand.as_any(),
        FocusExitCommand.as_any(),
    ];

    v_stack! {
        align = Align::BOTTOM_RIGHT;
        padding = 10;
        spacing = 8;
        items_align = Align::RIGHT;

        font_family = FontName::monospace();
        text_color = colors::GRAY;

        items = cmds.into_iter().map(|cmd| {
            text! {
                text = cmd.name_with_shortcut();

                when *#{cmd.enabled()} {
                    color = colors::WHITE;
                }
            }.boxed_wgt()
        }).collect::<WidgetVec>();
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
                } else if args.is_enabled_change() {
                    println!("focused {:?} enabled/disabled", args.new_focus.as_ref().unwrap());
                } else {
                    println!(
                        "{} -> {}",
                        inspect::focus(&args.prev_focus, ctx.services),
                        inspect::focus(&args.new_focus, ctx.services)
                    );
                }
            }),
        )
        .perm();
}

fn nested_focusables() -> impl Widget {
    button! {
        content = text("Nested Focusables");

        on_click = hn!(|ctx, args: &ClickArgs| {
            args.propagation().stop();
            Windows::req(ctx.services).focus_or_open("nested-focusables", |_| {
                window! {
                    title = "Focus Example - Nested Focusables";
                    // zero_ui::properties::inspector::show_center_points = true;
                    content_align = Align::CENTER;
                    content = v_stack! {
                        spacing = 10;
                        items = widgets![
                            nested_focusables_group('a'),
                            nested_focusables_group('b'),
                        ];
                    }
                }
            });
        })
    }
}
fn nested_focusables_group(g: char) -> impl Widget {
    h_stack! {
        align = Align::TOP;
        spacing = 10;
        items = (0..5).map(|n| nested_focusable(g, n, 0).boxed_wgt()).collect::<WidgetVec>()
    }
}
fn nested_focusable(g: char, column: u8, row: u8) -> impl Widget {
    let nested = text! {
        text = format!("Focusable {column}, {row}");
        margin = 5;
    };
    v_stack! {
        id = formatx!("focusable-{g}-{column}-{row}");
        padding = 2;

        items = if row == 5 {
            widget_vec![nested]
        } else {
            widget_vec![
                nested,
                nested_focusable(g, column, row + 1),
            ]
        };

        focusable = true;

        corner_radius = 5;
        border = 1, colors::RED.with_alpha(30.pct());
        background_color = colors::RED.with_alpha(20.pct());
        when self.is_focused {
            background_color = colors::GREEN;
        }
        when self.is_return_focus {
            border = 1, colors::LIME_GREEN;
        }
    }
}

#[cfg(debug_assertions)]
mod inspect {
    use super::*;
    use zero_ui::core::focus::WidgetInfoFocusExt;
    use zero_ui::core::inspector::WidgetInspectorInfo;

    pub fn focus(path: &Option<InteractionPath>, services: &mut Services) -> String {
        path.as_ref()
            .map(|p| {
                let frame = if let Ok(w) = Windows::req(services).widget_tree(p.window_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let widget = if let Some(w) = frame.get(p.widget_id()) {
                    w
                } else {
                    return format!("<{p}>");
                };
                let info = widget.instance().expect("expected debug info").borrow();

                if info.widget_name == "button" {
                    format!(
                        "button({})",
                        widget
                            .descendant_instance("text")
                            .expect("expected text in button")
                            .property("text")
                            .expect("expected text property")
                            .borrow()
                            .arg(0)
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
                    let focus_info = widget.as_focus_info(true, true);
                    if focus_info.is_alt_scope() {
                        format!("{}(is_alt_scope)", info.widget_name)
                    } else if focus_info.is_scope() {
                        format!("{}(is_scope)", info.widget_name)
                    } else {
                        format!("{}({})", info.widget_name, p.widget_id())
                    }
                }
            })
            .unwrap_or_else(|| "<none>".to_owned())
    }
}

#[cfg(not(debug_assertions))]
mod inspect {
    use super::*;

    pub fn focus(path: &Option<InteractionPath>, _: &mut Services) -> String {
        path.as_ref()
            .map(|p| format!("{:?}", p.widget_id()))
            .unwrap_or_else(|| "<none>".to_owned())
    }
}
