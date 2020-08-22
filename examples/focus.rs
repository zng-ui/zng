#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use zero_ui::{core::focus::TabIndex, prelude::*};

fn main() {
    App::default().run_window(|_| {
        window! {
            title: "Focus Example";
            content: v_stack! {
                spacing: 5.0;
                items: ui_vec![
                    example("Button 5", TabIndex(5)),
                    example("Button 4", TabIndex(3)),
                    example("Button 3", TabIndex(2)),
                    example("Button 1", TabIndex(0)),
                    example("Button 2", TabIndex(0)),
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
            println!("Clicked {}, {:?}", content, tab_index)
        };
    }
}
