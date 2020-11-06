#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use enclose::enclose;
use meval::eval_str;
use zero_ui::prelude::*;

fn main() {
    App::default().run_window(|_| {
        let calc = var(Calculator::default());
        window! {
            title: "Calculator";
            resizable: false;
            content: v_stack! {
                spacing: 5;
                items: ui_vec![
                    text! {
                        text: calc.map_ref(|c| c.result());
                        align: Alignment::RIGHT;
                        font_size: 32.pt();
                        color: calc.map(|c| c.color());
                    },
                    controls(calc.clone())
                ];
            };
            on_char_input: enclose! { (calc) move |ctx, args| {
                let char_ = args.character;
                calc.modify(ctx.vars, move |c|c.push(char_));
            }};
            // on_enter
            on_preview_key_down: move |ctx, args| {
                match args.key {
                    Some(VirtualKeyCode::Return) | Some(VirtualKeyCode::NumpadEnter) => {
                        calc.modify(ctx.vars, |c|c.eval());
                        args.stop_propagation();
                    },
                    _ => { }
                }
            };
        }
    })
}

fn controls(calc: RcVar<Calculator>) -> impl Widget {
    let b = |c| btn(calc.clone(), c);
    let btn_eq = btn_eval(calc.clone());
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

fn btn(calc: RcVar<Calculator>, c: char) -> impl Widget {
    button! {
        on_click: move |ctx, _| {
            calc.modify(ctx.vars, move |b| b.push(c))
        };
        content: text(c.to_string());
    }
}

fn btn_eval(calc: RcVar<Calculator>) -> impl Widget {
    button! {
        on_click: move |ctx, _| calc.modify(ctx.vars, |c|c.eval());
        content: text("=");
    }
}

#[derive(Default, Clone, Debug)]
struct Calculator {
    buffer: Text,
    error: bool,
}
impl Calculator {
    pub fn result(&self) -> &Text {
        if self.buffer.is_empty() {
            static ZERO: Text = Text::borrowed("0");
            &ZERO
        } else {
            &self.buffer
        }
    }

    pub fn color(&self) -> Rgba {
        if self.error {
            web_colors::RED
        } else {
            web_colors::WHITE
        }
    }

    fn char_is_valid(c: char) -> bool {
        c.is_digit(10) || ['.', '+', '-', '*', '/'].contains(&c)
    }

    pub fn push(&mut self, c: char) {
        if !Self::char_is_valid(c) {
            return;
        }

        if self.error {
            self.buffer.clear();
            self.error = false;
        }

        if self.buffer.is_empty() && c == '.' {
            self.buffer.to_mut().push_str("0.");
        } else {
            if !c.is_digit(10) && self.trailing_op() {
                self.buffer.to_mut().pop();
            }

            self.buffer.to_mut().push(c);
        }
    }

    fn trailing_op(&self) -> bool {
        self.buffer.chars().last().map(|c| !c.is_digit(10)).unwrap_or(false)
    }

    pub fn eval(&mut self) {
        use std::fmt::Write;

        let expr = if self.trailing_op() {
            &self.buffer[..self.buffer.len() - 1]
        } else {
            &self.buffer
        };

        if expr.is_empty() {
            self.buffer.clear();
            self.error = false;
        } else {
            match eval_str(expr) {
                Ok(new) => {
                    self.buffer.clear();
                    let _ = write!(&mut self.buffer.to_mut(), "{}", new);
                    self.error = false;
                }
                Err(_) => {
                    self.error = true;
                }
            }
        }
    }
}
