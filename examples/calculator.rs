#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use meval::eval_str;
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let buffer = var("");
        window! {
            title: "Calculator";
            content: v_stack!{
                spacing: 5;
                items: ui_vec![
                    text!{
                        text: buffer.clone();
                        font_size: 32.pt();
                    },
                    controls(buffer)
                ];
            };
        }
    })
}

fn controls(buffer: RcVar<Text>) -> impl Widget {
    let b = |c| btn(buffer.clone(), c);
    let btn_eq = btn_eval(buffer.clone());
    uniform_grid! {
        spacing: 2;
        columns: 4;
        items: ui_vec![
            b('7'), b('8'), b('9'), b('/'),
            b('4'), b('5'), b('6'), b('*'),
            b('1'), b('2'), b('3'), b('-'),
            b('0'), b('.'), btn_eq , b('+'),
        ];
    }
}

fn btn(buffer: RcVar<Text>, c: char) -> impl Widget {
    button! {
        on_click: move |a| {
            buffer.modify(a.ctx().vars, move |b| b.to_mut().push(c))
        };
        content: text(c.to_string());
    }
}

fn btn_eval(buffer: RcVar<Text>) -> impl Widget {
    button! {
        on_click: move |a| {
            buffer.modify(a.ctx().vars, move |b| eval(b.to_mut()))
        };
        content: text("=");
    }
}

fn eval(buffer: &mut String) {
    use std::fmt::Write;
    let eval_result = eval_str(&buffer).unwrap_or(0.0);
    buffer.clear();
    let _ = write!(buffer, "{}", eval_result);
}