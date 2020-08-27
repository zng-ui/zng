#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Focus Example";
            on_focus_changed: |a| {
                let args = a.args();
                let ctx = a.ctx();

                if args.is_hightlight_changed() {
                    println!("highlight: {}", args.highlight);
                } else if args.is_widget_move() {
                    println!("focused {:?} moved", args.new_focus.as_ref().unwrap());
                } else {
                    println!("{:<18} -> {}", inspect::focus(&args.prev_focus, ctx), inspect::focus(&args.new_focus, ctx));
                }

            };
            content_align: unset!;
            content: v_stack! {
                items: ui_vec![
                    h_stack! {
                        alt_focus_scope: true;
                        spacing: 5.0;
                        margin: 5.0;
                        items: ui_vec![
                            example("alt", TabIndex::AUTO),
                            example("scope", TabIndex::AUTO),
                        ];
                    },
                    v_stack! {
                        focus_scope: true;
                        focus_shortcut: shortcut!(T);
                        margin: (50.0, 0.0, 0.0, 0.0);
                        align: Alignment::CENTER;
                        spacing: 5.0;
                        items: ui_vec![
                            text! { text: "TabIndex (T)"; font_weight: FontWeight::BOLD; align: Alignment::CENTER; },
                            example("Button 5", TabIndex(5)),
                            example("Button 4", TabIndex(3)),
                            example("Button 3", TabIndex(2)),
                            example("Button 1", TabIndex(0)),
                            example("Button 2", TabIndex(0)),
                        ];
                    }
                ];
            };
        }
    })
}

fn example(content: impl Into<Text>, tab_index: TabIndex) -> impl Widget {
    let content = content.into();
    button! {
        content: text(content.clone());
        tab_index;
        on_click: move |_| {
            println!("Clicked {} {:?}", content, tab_index)
        };
    }
}

#[cfg(debug_assertions)]
mod inspect {
    use super::*;
    use zero_ui::core::context::WidgetContext;
    use zero_ui::core::debug::WidgetDebugInfo;
    use zero_ui::core::focus::WidgetInfoFocusExt;

    pub fn focus(path: &Option<WidgetPath>, ctx: &mut WidgetContext) -> String {
        path.as_ref()
            .map(|p| {
                let window = ctx.services.req::<Windows>().window(p.window_id()).expect("expected window");
                let frame = window.frame_info();
                let widget = frame.get(p).expect("expected widget");
                let info = widget.instance().expect("expected debug info").borrow();

                if info.widget_name == "button" {
                    let text_wgt = widget.descendants().next().expect("expected text in button");
                    let info = text_wgt.instance().expect("expected debug info").borrow();
                    format!(
                        "button({})",
                        info.captured_new_child
                            .iter()
                            .find(|p| p.property_name == "text")
                            .expect("expected text in capture_new")
                            .args[0]
                            .value
                    )
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
    use zero_ui::core::context::WidgetContext;

    pub fn focus(path: &Option<WidgetPath>, ctx: &mut WidgetContext) -> String {
        path.as_ref()
            .map(|p| format!("{:?}", p.widget_id()))
            .unwrap_or_else(|| "<none>".to_owned())
    }
}
