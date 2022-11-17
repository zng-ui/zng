#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use meval::eval_str;
use std::convert::TryInto;
use zero_ui::prelude::*;

use zero_ui_view_prebuilt as zero_ui_view;

fn main() {
    examples_util::print_info();
    zero_ui_view::init();

    //let rec = examples_util::record_profile("calculator");

    // zero_ui_view::run_same_process(app_main);
    app_main();

    //rec.finish();
}

fn app_main() {
    App::default().run_window(|ctx| {
        set_fallback_font(ctx);

        let calc = var(Calculator::default());
        window! {
            title = "Calculator";
            // zero_ui::properties::inspector::show_bounds = true;
            resizable = false;
            auto_size = true;
            padding = 5;
            child = v_stack! {
                spacing = 5;
                children = ui_list![
                    text! {
                        txt = calc.map_ref(|c| c.text());
                        align = Align::RIGHT;
                        font_size = 32.pt();

                        when #{calc.clone()}.error() {
                            txt_color = colors::RED;
                        }
                    },
                    controls(calc)
                ];
            };
        }
    })
}

fn controls(calc: RcVar<Calculator>) -> impl UiNode {
    let bn = |c| btn(calc.clone(), c);
    let b_squre = btn_square(calc.clone());
    let b_sroot = btn_square_root(calc.clone());
    let b_clear = btn_clear(calc.clone());
    let b_back = btn_backspace(calc.clone());
    let b_equal = btn_eval(calc.clone());

    uniform_grid! {
        spacing = 2;
        columns = 4;
        font_size = 14.pt();
        children = ui_list![
            b_squre,  b_sroot,  b_clear,  b_back,
            bn('7'),  bn('8'),  bn('9'),  bn('/'),
            bn('4'),  bn('5'),  bn('6'),  bn('*'),
            bn('1'),  bn('2'),  bn('3'),  bn('-'),
            bn('0'),  bn('.'),  b_equal,  bn('+'),
        ];
    }
}

fn btn_square(calc: RcVar<Calculator>) -> impl UiNode {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, | c|c.to_mut().square()));
        child = text("x²");
    }
}

fn btn_square_root(calc: RcVar<Calculator>) -> impl UiNode {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, | c|c.to_mut().square_root()));
        child = text("√x");
    }
}

fn btn_clear(calc: RcVar<Calculator>) -> impl UiNode {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, | c|c.to_mut().clear()));
        click_shortcut = shortcut!(Escape);
        child = text("C");
    }
}

fn btn_backspace(calc: RcVar<Calculator>) -> impl UiNode {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, |c|c.to_mut().backspace()));
        click_shortcut = shortcut!(Backspace);
        child = text("⌫");
    }
}

fn btn(calc: RcVar<Calculator>, c: char) -> impl UiNode {
    button! {
        on_click = hn!(|ctx, _| {
            calc.modify(ctx.vars, move |b| b.to_mut().push(c))
        });
        click_shortcut = {
            let shortcuts: Shortcuts = c.try_into().unwrap_or_default();
            assert!(!shortcuts.0.is_empty());
            shortcuts
        };
        child = text(c.to_string());
    }
}

fn btn_eval(calc: RcVar<Calculator>) -> impl UiNode {
    button! {
        on_click = hn!(|ctx, _| calc.modify(ctx.vars, |c|c.to_mut().eval()));
        click_shortcut = vec![shortcut!(Enter), shortcut!(NumpadEnter), shortcut!(Equals)];
        child = text("=");
    }
}

#[derive(Default, Clone, Debug)]
struct Calculator {
    buffer: Text,
    error: bool,
}
impl Calculator {
    pub fn text(&self) -> &Text {
        if self.buffer.is_empty() {
            static ZERO: Text = Text::from_static("0");
            &ZERO
        } else {
            &self.buffer
        }
    }

    pub fn error(&self) -> bool {
        self.error
    }

    fn char_is_valid(c: char) -> bool {
        c.is_ascii_digit() || ['.', '+', '-', '*', '/'].contains(&c)
    }

    pub fn push(&mut self, c: char) {
        if !Self::char_is_valid(c) {
            return;
        }

        if self.error {
            self.buffer.clear();
            self.error = false;
        }

        if self.buffer.is_empty() && !c.is_ascii_digit() && c != '-' {
            let b = self.buffer.to_mut();
            b.push('0');
            b.push(c);
        } else {
            if !c.is_ascii_digit() && self.trailing_op() {
                self.buffer.to_mut().pop();
            }

            self.buffer.to_mut().push(c);
        }
    }

    fn trailing_op(&self) -> bool {
        self.buffer.chars().last().map(|c| !c.is_ascii_digit() && c != ')').unwrap_or(false)
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.error = false;
    }

    pub fn backspace(&mut self) {
        self.buffer.pop();
        self.error = false;
    }

    pub fn square(&mut self) {
        if self.error {
            self.clear()
        } else if !self.buffer.is_empty() {
            self.buffer = formatx!("({})^2", self.buffer)
        }
    }

    pub fn square_root(&mut self) {
        if self.error {
            self.clear()
        } else if !self.buffer.is_empty() {
            self.buffer = formatx!("sqrt({})", self.buffer)
        }
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
                    if new.is_finite() {
                        self.buffer.clear();
                        let _ = write!(&mut self.buffer.to_mut(), "{new}");
                        self.error = false;
                    } else {
                        eprintln!("Result not finite: {new}");
                        self.error = true;
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    self.error = true;
                }
            }
        }
    }
}

/// set custom fallback font for the ⌫ symbol.
fn set_fallback_font(ctx: &mut WindowContext) {
    use zero_ui::core::text::*;

    let fonts = Fonts::req(ctx.services);
    let und = lang!(und);
    if fonts
        .list(
            &FontNames::system_ui(&und),
            FontStyle::Normal,
            FontWeight::NORMAL,
            FontStretch::NORMAL,
            &und,
        )
        .iter()
        .all(|f| f.font_kit().glyph_for_char('⌫').is_none())
    {
        // OS UI and fallback fonts do not support `⌫`, load custom font that does.

        static FALLBACK: &[u8] = include_bytes!("res/calculator/notosanssymbols2-regular-subset.ttf");
        let fallback = zero_ui::core::text::CustomFont::from_bytes("fallback", FontDataRef::from_static(FALLBACK), 0);

        fonts.register(fallback).unwrap();
        fonts.generics_mut().set_fallback(und, "fallback");
    }
}
