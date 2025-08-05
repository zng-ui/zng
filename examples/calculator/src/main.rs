//! Simple calculator, demonstrates Grid layout, data context.

use std::convert::TryInto;
use zng::{
    gesture::{Shortcuts, click_shortcut},
    prelude::*,
};

fn main() {
    zng::env::init!();

    APP.defaults().run_window(async {
        set_fallback_font().await;

        Window! {
            title = "Calculator";
            data = var(Calculator::default());
            resizable = false;
            auto_size = true;
            enabled_buttons = !window::WindowButton::MAXIMIZE;
            padding = 5;
            child = Stack! {
                direction = StackDirection::top_to_bottom();
                spacing = 5;
                children = ui_vec![
                    Text! {
                        txt = DATA.req::<Calculator>().map(|c| c.text());
                        layout::align = Align::RIGHT;
                        font_size = 32.pt();

                        when #{DATA.req::<Calculator>()}.error() {
                            font_color = colors::RED;
                        }
                    },
                    controls()
                ];
            };
        }
    })
}

fn controls() -> impl UiNode {
    let bn = btn;
    let b_squre = btn_square();
    let b_sroot = btn_square_root();
    let b_clear = btn_clear();
    let b_back = btn_backspace();
    let b_equal = btn_eval();

    #[rustfmt::skip]
    let buttons = ui_vec![
        b_squre, b_sroot, b_clear, b_back,
        bn('7'), bn('8'), bn('9'), bn('/'),
        bn('4'), bn('5'), bn('6'), bn('*'),
        bn('1'), bn('2'), bn('3'), bn('-'),
        bn('0'), bn('.'), b_equal, bn('+'),
    ];

    Grid! {
        spacing = 2;
        columns = ui_vec![grid::Column!(1.lft()); 4];
        auto_grow_fn = wgt_fn!(|_| grid::Row!(1.lft()));
        text::font_size = 14.pt();
        cells = buttons;
    }
}

fn btn_square() -> impl UiNode {
    Button! {
        grid::cell::at = grid::cell::AT_AUTO;
        on_click = hn!(|_| DATA.req::<Calculator>().modify(|c| c.square()));
        child = Text!("x²");
    }
}

fn btn_square_root() -> impl UiNode {
    Button! {
        grid::cell::at = grid::cell::AT_AUTO;
        on_click = hn!(|_| DATA.req::<Calculator>().modify(|c| c.square_root()));
        child = Text!("√x");
    }
}

fn btn_clear() -> impl UiNode {
    Button! {
        grid::cell::at = grid::cell::AT_AUTO;
        on_click = hn!(|_| DATA.req::<Calculator>().modify(|c| c.clear()));
        click_shortcut = shortcut!(Escape);
        child = Text!("C");
    }
}

fn btn_backspace() -> impl UiNode {
    Button! {
        grid::cell::at = grid::cell::AT_AUTO;
        on_click = hn!(|_| DATA.req::<Calculator>().modify(|c| c.backspace()));
        click_shortcut = shortcut!(Backspace);
        child = Text!("⌫");
    }
}

fn btn(c: char) -> impl UiNode {
    Button! {
        grid::cell::at = grid::cell::AT_AUTO;
        on_click = hn!(|_| {
            DATA.req::<Calculator>().modify(move |b| b.push(c));
        });
        click_shortcut = {
            let shortcuts: Shortcuts = c.try_into().unwrap_or_default();
            assert!(!shortcuts.0.is_empty());
            shortcuts
        };
        child = Text!(c.to_string());
    }
}

fn btn_eval() -> impl UiNode {
    Button! {
        grid::cell::at = grid::cell::AT_AUTO;
        on_click = hn!(|_| DATA.req::<Calculator>().modify(|c| c.eval()));
        click_shortcut = vec![shortcut!(Enter), shortcut!('=')];
        child = Text!("=");
    }
}

#[derive(Default, Clone, Debug, PartialEq)]
struct Calculator {
    buffer: Txt,
    error: bool,
}
impl Calculator {
    pub fn text(&self) -> Txt {
        if self.buffer.is_empty() {
            Txt::from_static("0")
        } else {
            self.buffer.clone()
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
            match mexprp::eval::<f64>(expr) {
                Ok(new) => {
                    // square-root of a positive number is both a positive number and a negative number
                    // we only want the principal square root though, the positive one, which should be the first answer
                    let new = new.to_vec()[0];
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
async fn set_fallback_font() {
    use zng::font::*;
    let und = lang!(und);

    let shaped_icon = FONTS
        .list(
            &FontNames::system_ui(&und),
            FontStyle::Normal,
            FontWeight::NORMAL,
            FontStretch::NORMAL,
            &und,
        )
        .wait_rsp()
        .await
        .sized(layout::Px(11), vec![])
        .shape_text(&SegmentedText::new("⌫", layout::LayoutDirection::LTR), &TextShapingArgs::default());

    if shaped_icon.is_empty() || shaped_icon.glyphs().flat_map(|g| g.1).any(|g| g.index == 0) {
        // OS UI and fallback fonts do not support `⌫`, load custom font that does.

        static FALLBACK: &[u8] = include_bytes!("../res/notosanssymbols2-regular-subset.ttf");
        let fallback = CustomFont::from_bytes("fallback", FontDataRef::from_static(FALLBACK), 0);

        FONTS.register(fallback).wait_rsp().await.unwrap();
        FONTS.generics().set_fallback(und, "fallback");
    }
}
