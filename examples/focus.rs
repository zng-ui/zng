#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::{core::focus::TabIndex, prelude::*};

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Focus Example";
            on_focus_changed: |a| {
                fn id(path: &Option<WidgetPath>) -> String {
                    path.as_ref().map(|p|format!("{:?}", p.widget_id())).unwrap_or_else(||"None".to_owned())
                }
                let args = a.args();
                println!("focus changed: {} -> {}", id(&args.prev_focus), id(&args.new_focus));
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
                            tab_index_btn(TabIndex(5)),
                            tab_index_btn(TabIndex(3)),
                            tab_index_btn(TabIndex(2)),
                            tab_index_btn(TabIndex(0)),
                            tab_index_btn(TabIndex(0)),
                        ];
                    }
                ];
            };
        }
    })
}

fn tab_index_btn(tab_index: TabIndex) -> impl Widget {
    example(format!("{:?}", tab_index), tab_index)
}

fn example(content: impl Into<Text>, tab_index: TabIndex) -> impl Widget {
    let content = content.into();
    button! {
        content: text(content.clone());
        tab_index;
        on_click: move |_| {
            println!("Clicked {}", content)
        };
    }
}
