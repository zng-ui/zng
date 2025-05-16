#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! ANSI text widget, properties and nodes.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_ext_font::*;
use zng_wgt::{prelude::*, *};
use zng_wgt_fill::*;
use zng_wgt_filter::*;
use zng_wgt_input::{CursorIcon, cursor};
use zng_wgt_scroll::{LazyMode, lazy};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_text::*;

#[doc(hidden)]
pub use zng_wgt_text::__formatx;

/// Render text styled using ANSI escape sequences.
///
/// Supports color, weight, italic and more, see [`AnsiStyle`] for the full style supported.
#[widget($crate::AnsiText {
    ($txt:literal) => {
        txt = $crate::__formatx!($txt);
    };
    ($txt:expr) => {
        txt = $txt;
    };
    ($txt:tt, $($format:tt)*) => {
        txt = $crate::__formatx!($txt, $($format)*);
    };
})]
#[rustfmt::skip]
pub struct AnsiText(
    FontMix<
    TextSpacingMix<
    ParagraphMix<
    LangMix<
    WidgetBase
    >>>>
);
impl AnsiText {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            font_family = ["JetBrains Mono", "Consolas", "monospace"];
            rich_text = true;

            when #txt_selectable {
                cursor = CursorIcon::Text;
            }
        };

        self.widget_builder().push_build_action(|wgt| {
            let txt = wgt.capture_var_or_default(property_id!(txt));
            let child = ansi_node(txt);
            wgt.set_child(child.boxed());
        });
    }

    widget_impl! {
        /// ANSI text.
        pub txt(text: impl IntoVar<Txt>);

        /// Enable text selection, copy.
        ///
        /// Note that the copy is only in plain text, without the ANSI escape codes.
        pub zng_wgt_text::txt_selectable(enabled: impl IntoVar<bool>);
    }
}

pub use ansi_parse::*;
mod ansi_parse {

    use super::*;

    /// Represents a segment of ANSI styled text that shares the same style.
    #[derive(Debug)]
    #[non_exhaustive]
    pub struct AnsiTxt<'a> {
        /// Text run.
        pub txt: &'a str,
        /// Text style.
        pub style: AnsiStyle,
    }

    /// Represents the ANSI style of a text run.
    ///
    /// See [`AnsiText`](struct@super::AnsiText) for more details.
    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    #[non_exhaustive]
    pub struct AnsiStyle {
        /// Background color.
        pub background_color: AnsiColor,
        /// Font color.
        pub color: AnsiColor,
        /// Font weight.
        pub weight: AnsiWeight,
        /// Font italic.
        pub italic: bool,
        /// Underline.
        pub underline: bool,
        /// Strikethrough.
        pub strikethrough: bool,
        /// Negative color.
        pub invert_color: bool,
        /// Visibility.
        pub hidden: bool,
        /// Blink animation.
        pub blink: bool,
    }
    impl Default for AnsiStyle {
        fn default() -> Self {
            Self {
                background_color: AnsiColor::Black,
                color: AnsiColor::White,
                weight: Default::default(),
                italic: false,
                underline: false,
                strikethrough: false,
                invert_color: false,
                hidden: false,
                blink: false,
            }
        }
    }

    /// Named ANSI color.
    ///
    /// See [`AnsiStyle`] for more details.
    #[allow(missing_docs)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum AnsiColor {
        Black,
        Red,
        Green,
        Yellow,
        Blue,
        Magenta,
        Cyan,
        White,
        /// Gray
        BrightBlack,
        BrightRed,
        BrightGreen,
        BrightYellow,
        BrightBlue,
        BrightMagenta,
        BrightCyan,
        BrightWhite,
        /// 8-bit lookup.
        Ansi256(u8),
        /// RGB
        TrueColor(u8, u8, u8),
    }
    impl_from_and_into_var! {
        fn from(color: AnsiColor) -> Rgba {
            match color {
                AnsiColor::Black => rgb(0, 0, 0),
                AnsiColor::Red => rgb(205, 49, 49),
                AnsiColor::Green => rgb(13, 188, 121),
                AnsiColor::Yellow => rgb(229, 229, 16),
                AnsiColor::Blue => rgb(36, 114, 200),
                AnsiColor::Magenta => rgb(188, 63, 188),
                AnsiColor::Cyan => rgb(17, 168, 205),
                AnsiColor::White => rgb(229, 229, 229),
                AnsiColor::BrightBlack => rgb(102, 102, 102),
                AnsiColor::BrightRed => rgb(241, 76, 76),
                AnsiColor::BrightGreen => rgb(35, 209, 139),
                AnsiColor::BrightYellow => rgb(245, 245, 67),
                AnsiColor::BrightBlue => rgb(59, 142, 234),
                AnsiColor::BrightMagenta => rgb(214, 112, 214),
                AnsiColor::BrightCyan => rgb(41, 184, 219),
                AnsiColor::BrightWhite => rgb(229, 229, 229),
                AnsiColor::Ansi256(c) => {
                    let (r, g, b) = X_TERM_256[c as usize];
                    rgb(r, g, b)
                }
                AnsiColor::TrueColor(r, g, b) => rgb(r, g, b),
            }
        }
    }

    /// Font weight defined by ANSI escape codes.
    ///
    /// See [`AnsiStyle`] for more details.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum AnsiWeight {
        /// Normal.
        Normal,
        /// Bold.
        Bold,
        /// Light.
        Faint,
    }
    impl Default for AnsiWeight {
        fn default() -> Self {
            Self::Normal
        }
    }
    impl_from_and_into_var! {
        fn from(weight: AnsiWeight) -> FontWeight {
            match weight {
                AnsiWeight::Normal => FontWeight::NORMAL,
                AnsiWeight::Bold => FontWeight::BOLD,
                AnsiWeight::Faint => FontWeight::LIGHT,
            }
        }
    }

    /// Iterator that parses ANSI escape codes.
    ///
    /// This is the pull style parser used internally by the [`AnsiText!`] widget.
    ///
    /// [`AnsiText!`]: struct@crate::AnsiText
    pub struct AnsiTextParser<'a> {
        source: &'a str,
        /// Current style.
        pub style: AnsiStyle,
    }
    impl<'a> AnsiTextParser<'a> {
        /// New parsing iterator.
        pub fn new(source: &'a str) -> Self {
            Self {
                source,
                style: AnsiStyle::default(),
            }
        }
    }
    impl<'a> Iterator for AnsiTextParser<'a> {
        type Item = AnsiTxt<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            const CSI: &str = "\x1b[";

            fn is_esc_end(byte: u8) -> bool {
                (0x40..=0x7e).contains(&byte)
            }

            loop {
                if self.source.is_empty() {
                    return None;
                } else if let Some(source) = self.source.strip_prefix(CSI) {
                    let mut esc_end = 0;
                    while esc_end < source.len() && !is_esc_end(source.as_bytes()[esc_end]) {
                        esc_end += 1;
                    }
                    esc_end += 1;

                    let (esc, source) = source.split_at(esc_end);

                    let esc = &esc[..(esc.len() - 1)];
                    self.style.set(esc);

                    self.source = source;
                    continue;
                } else if let Some(i) = self.source.find(CSI) {
                    let (txt, source) = self.source.split_at(i);
                    self.source = source;
                    return Some(AnsiTxt {
                        txt,
                        style: self.style.clone(),
                    });
                } else {
                    return Some(AnsiTxt {
                        txt: std::mem::take(&mut self.source),
                        style: self.style.clone(),
                    });
                }
            }
        }
    }

    impl AnsiStyle {
        fn set(&mut self, esc_codes: &str) {
            let mut esc_codes = esc_codes.split(';');
            while let Some(code) = esc_codes.next() {
                match code {
                    "0" => *self = Self::default(),
                    "1" => self.weight = AnsiWeight::Bold,
                    "2" => self.weight = AnsiWeight::Faint,
                    "3" => self.italic = true,
                    "4" => self.underline = true,
                    "5" => self.blink = true,
                    "7" => self.invert_color = true,
                    "8" => self.hidden = true,
                    "9" => self.strikethrough = true,
                    "22" => self.weight = AnsiWeight::Normal,
                    "23" => self.italic = false,
                    "24" => self.underline = false,
                    "25" => self.blink = false,
                    "27" => self.invert_color = false,
                    "28" => self.hidden = false,
                    "29" => self.strikethrough = false,
                    "30" => self.color = AnsiColor::Black,
                    "31" => self.color = AnsiColor::Red,
                    "32" => self.color = AnsiColor::Green,
                    "33" => self.color = AnsiColor::Yellow,
                    "34" => self.color = AnsiColor::Blue,
                    "35" => self.color = AnsiColor::Magenta,
                    "36" => self.color = AnsiColor::Cyan,
                    "37" => self.color = AnsiColor::White,
                    "40" => self.color = AnsiColor::Black,
                    "41" => self.color = AnsiColor::Red,
                    "42" => self.color = AnsiColor::Green,
                    "43" => self.color = AnsiColor::Yellow,
                    "44" => self.color = AnsiColor::Blue,
                    "45" => self.color = AnsiColor::Magenta,
                    "46" => self.color = AnsiColor::Cyan,
                    "47" => self.color = AnsiColor::White,
                    "90" => self.color = AnsiColor::BrightBlack,
                    "91" => self.color = AnsiColor::BrightRed,
                    "92" => self.color = AnsiColor::BrightGreen,
                    "93" => self.color = AnsiColor::BrightYellow,
                    "94" => self.color = AnsiColor::BrightBlue,
                    "95" => self.color = AnsiColor::BrightMagenta,
                    "96" => self.color = AnsiColor::BrightCyan,
                    "97" => self.color = AnsiColor::BrightWhite,
                    "100" => self.background_color = AnsiColor::BrightBlack,
                    "101" => self.background_color = AnsiColor::BrightRed,
                    "102" => self.background_color = AnsiColor::BrightGreen,
                    "103" => self.background_color = AnsiColor::BrightYellow,
                    "104" => self.background_color = AnsiColor::BrightBlue,
                    "105" => self.background_color = AnsiColor::BrightMagenta,
                    "106" => self.background_color = AnsiColor::BrightCyan,
                    "107" => self.background_color = AnsiColor::BrightWhite,
                    "38" | "48" => {
                        let target = if code == "38" {
                            &mut self.color
                        } else {
                            &mut self.background_color
                        };
                        match esc_codes.next() {
                            Some("5") => {
                                let c = esc_codes.next().and_then(|c| c.parse().ok()).unwrap_or(0);
                                *target = AnsiColor::Ansi256(c)
                            }
                            Some("2") => {
                                let r = esc_codes.next().and_then(|c| c.parse().ok()).unwrap_or(0);
                                let g = esc_codes.next().and_then(|c| c.parse().ok()).unwrap_or(0);
                                let b = esc_codes.next().and_then(|c| c.parse().ok()).unwrap_or(0);

                                *target = AnsiColor::TrueColor(r, g, b);
                            }
                            _ => {}
                        }
                    }
                    _ => (),
                }
            }
        }
    }
}

pub use ansi_fn::*;
mod ansi_fn {
    use std::time::Duration;

    use super::{AnsiColor, AnsiStyle, AnsiWeight};

    use super::*;

    /// Arguments for a widget function for an ANSI styled text fragment.
    ///
    /// See [`TEXT_FN_VAR`] for more details.
    #[non_exhaustive]
    pub struct TextFnArgs {
        /// The text.
        pub txt: Txt,
        /// The ANSI style.
        pub style: AnsiStyle,
    }
    impl TextFnArgs {
        /// New from text and style.
        pub fn new(txt: impl Into<Txt>, style: AnsiStyle) -> Self {
            Self { txt: txt.into(), style }
        }
    }

    /// Arguments for a widget function for a text line.
    ///
    /// See [`LINE_FN_VAR`] for more details.
    #[non_exhaustive]
    pub struct LineFnArgs {
        /// Zero-counted global index of this line.
        pub index: u32,
        /// Zero-counted index of this line in the parent page.
        pub page_index: u32,
        /// Text segment widgets, generated by [`TEXT_FN_VAR`].
        pub text: UiVec,
    }

    impl LineFnArgs {
        /// New args.
        pub fn new(index: u32, page_index: u32, text: UiVec) -> Self {
            Self { index, page_index, text }
        }
    }

    /// Arguments for a widget function for a stack of lines.
    ///
    /// See [`PAGE_FN_VAR`] for more details.
    #[non_exhaustive]
    pub struct PageFnArgs {
        /// Zero-counted index of this page.
        pub index: u32,

        /// Line widgets, generated by [`LINE_FN_VAR`].
        pub lines: UiVec,
    }

    impl PageFnArgs {
        /// New args.
        pub fn new(index: u32, lines: UiVec) -> Self {
            Self { index, lines }
        }
    }

    /// Arguments for a widget function for a stack of pages.
    ///
    /// See [`PANEL_FN_VAR`] for more details.
    #[non_exhaustive]
    pub struct PanelFnArgs {
        /// Page widgets, generated by [`PAGE_FN_VAR`].
        pub pages: UiVec,
    }

    impl PanelFnArgs {
        /// New args.
        pub fn new(pages: UiVec) -> Self {
            Self { pages }
        }
    }

    context_var! {
        /// Widget function for [`TextFnArgs`].
        ///
        /// The returned widgets are layout by the [`LINE_FN_VAR`]. The default view is [`default_text_fn`].
        pub static TEXT_FN_VAR: WidgetFn<TextFnArgs> = wgt_fn!(|args: TextFnArgs| { default_text_fn(args) });

        /// Widget function for [`LineFnArgs`].
        ///
        /// The returned widgets are layout by the [`PAGE_FN_VAR`]. The default view is [`default_line_fn`].
        pub static LINE_FN_VAR: WidgetFn<LineFnArgs> = wgt_fn!(|args: LineFnArgs| { default_line_fn(args) });

        /// Widget function for [`PageFnArgs`].
        ///
        /// The returned widgets are layout by the [`PANEL_FN_VAR`] widget. The default view is [`default_page_fn`].
        pub static PAGE_FN_VAR: WidgetFn<PageFnArgs> = wgt_fn!(|args: PageFnArgs| { default_page_fn(args) });

        /// Widget function for [`PanelFnArgs`].
        ///
        /// The returned view is the [`AnsiText!`] child. The default is [`default_panel_fn`].
        ///
        /// [`AnsiText!`]: struct@super::AnsiText
        pub static PANEL_FN_VAR: WidgetFn<PanelFnArgs> = wgt_fn!(|args: PanelFnArgs| { default_panel_fn(args) });

        /// Duration the ANSI blink animation keeps the text visible for.
        ///
        /// Set to `ZERO` or `MAX` to disable animation.
        pub static BLINK_INTERVAL_VAR: Duration = Duration::ZERO;

        /// Maximum number of lines per [`PAGE_FN_VAR`].
        ///
        /// Is `200` by default.
        pub static LINES_PER_PAGE_VAR: u32 = 200;
    }

    /// Default [`TEXT_FN_VAR`].
    ///
    /// This view is configured by contextual variables like [`BLINK_INTERVAL_VAR`] and all text variables that are
    /// not overridden by the ANSI style, like the font.
    ///
    /// Returns a `Text!` with the text and style.
    pub fn default_text_fn(args: TextFnArgs) -> impl UiNode {
        let mut text = Text::widget_new();

        widget_set! {
            &mut text;
            txt = args.txt;
        }

        if args.style.background_color != AnsiColor::Black {
            widget_set! {
                &mut text;
                background_color = args.style.background_color;
            }
        }
        if args.style.color != AnsiColor::White {
            widget_set! {
                &mut text;
                font_color = args.style.color;
            }
        }

        if args.style.weight != AnsiWeight::Normal {
            widget_set! {
                &mut text;
                font_weight = args.style.weight;
            }
        }
        if args.style.italic {
            widget_set! {
                &mut text;
                font_style = FontStyle::Italic;
            }
        }

        if args.style.underline {
            widget_set! {
                &mut text;
                underline = 1, LineStyle::Solid;
            }
        }
        if args.style.strikethrough {
            widget_set! {
                &mut text;
                strikethrough = 1, LineStyle::Solid;
            }
        }

        if args.style.invert_color {
            widget_set! {
                &mut text;
                invert_color = true;
            }
        }

        if args.style.hidden {
            widget_set! {
                &mut text;
                visibility = Visibility::Hidden;
            }
        }
        if args.style.blink && !args.style.hidden {
            let opacity = var(1.fct());

            let interval = BLINK_INTERVAL_VAR.get();
            if interval != Duration::ZERO && interval != Duration::MAX {
                opacity.step_oci(0.fct(), interval).perm();

                widget_set! {
                    &mut text;
                    opacity;
                }
            }
        }

        text.widget_build()
    }

    /// Default [`LINE_FN_VAR`].
    ///
    /// Returns a `Wrap!` for text with multiple segments, or returns the single segment, or an empty text.
    pub fn default_line_fn(mut args: LineFnArgs) -> impl UiNode {
        use crate::prelude::*;

        if args.text.is_empty() {
            Text!("").boxed()
        } else if args.text.len() == 1 {
            args.text.remove(0)
        } else {
            Stack! {
                rich_text = true;
                direction = StackDirection::start_to_end();
                children = args.text;
            }
            .boxed()
        }
    }

    /// Default [`PAGE_FN_VAR`].
    ///
    /// Returns a `Stack!` for multiple lines, or return the single line, or a nil node.
    pub fn default_page_fn(mut args: PageFnArgs) -> impl UiNode {
        use crate::prelude::*;

        if args.lines.is_empty() {
            NilUiNode.boxed()
        } else if args.lines.len() == 1 {
            args.lines.remove(0)
        } else {
            let len = args.lines.len();
            Stack! {
                rich_text = true;
                direction = StackDirection::top_to_bottom();
                children = args.lines;
                lazy = LazyMode::lazy_vertical(wgt_fn!(|_| {
                    let height_sample = zng_wgt_text::node::line_placeholder(50);
                    zng_wgt_stack::lazy_sample(len, StackDirection::top_to_bottom(), 0, height_sample)
                }));
            }
            .boxed()
        }
    }

    /// Default [`PANEL_FN_VAR`].
    ///
    /// Returns a `Stack!` for multiple pages, or returns the single page, or a nil node.
    pub fn default_panel_fn(mut args: PanelFnArgs) -> impl UiNode {
        use crate::prelude::*;

        if args.pages.is_empty() {
            NilUiNode.boxed()
        } else if args.pages.len() == 1 {
            args.pages.remove(0)
        } else {
            Stack! {
                rich_text = true;
                direction = StackDirection::top_to_bottom();
                children = args.pages;
            }
            .boxed()
        }
    }

    /// ANSI blink animation interval.
    ///
    /// Set to `ZERO` to disable the blink animation.
    ///
    /// Sets the [`BLINK_INTERVAL_VAR`].
    #[property(CONTEXT, default(BLINK_INTERVAL_VAR), widget_impl(AnsiText))]
    pub fn blink_interval(child: impl UiNode, interval: impl IntoVar<Duration>) -> impl UiNode {
        with_context_var(child, BLINK_INTERVAL_VAR, interval)
    }

    /// Widget function that converts [`TextFnArgs`] to widgets.
    ///
    /// Sets the [`TEXT_FN_VAR`].
    #[property(CONTEXT, default(TEXT_FN_VAR), widget_impl(AnsiText))]
    pub fn text_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<TextFnArgs>>) -> impl UiNode {
        with_context_var(child, TEXT_FN_VAR, wgt_fn)
    }

    /// Widget function that converts [`LineFnArgs`] to widgets.
    ///
    /// Sets the [`LINE_FN_VAR`].
    #[property(CONTEXT, default(LINE_FN_VAR), widget_impl(AnsiText))]
    pub fn line_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<LineFnArgs>>) -> impl UiNode {
        with_context_var(child, LINE_FN_VAR, wgt_fn)
    }

    /// Widget function that converts [`PageFnArgs`] to widgets.
    ///
    /// A *page* is a stack of a maximum of [`lines_per_page`], the text is split in pages mostly for performance reasons.
    ///
    /// Sets the [`PAGE_FN_VAR`].
    ///
    /// [`lines_per_page`]: fn@lines_per_page
    #[property(CONTEXT, default(PAGE_FN_VAR), widget_impl(AnsiText))]
    pub fn page_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<PageFnArgs>>) -> impl UiNode {
        with_context_var(child, PAGE_FN_VAR, wgt_fn)
    }

    /// Widget function that converts [`PanelFnArgs`] to widgets.
    #[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(AnsiText))]
    pub fn panel_fn(child: impl UiNode, wgt_fn: impl IntoVar<WidgetFn<PanelFnArgs>>) -> impl UiNode {
        with_context_var(child, PANEL_FN_VAR, wgt_fn)
    }

    /// Maximum number of lines per page view.
    ///
    /// Sets the [`LINES_PER_PAGE_VAR`].
    #[property(CONTEXT, default(LINES_PER_PAGE_VAR), widget_impl(AnsiText))]
    pub fn lines_per_page(child: impl UiNode, count: impl IntoVar<u32>) -> impl UiNode {
        with_context_var(child, LINES_PER_PAGE_VAR, count)
    }
}

fn generate_ansi(txt: &impl Var<Txt>) -> BoxedUiNode {
    use ansi_fn::*;
    use std::mem;

    txt.with(|txt| {
        let text_fn = TEXT_FN_VAR.get();
        let line_fn = LINE_FN_VAR.get();
        let page_fn = PAGE_FN_VAR.get();
        let panel_fn = PANEL_FN_VAR.get();
        let lines_per_page = LINES_PER_PAGE_VAR.get() as usize;

        let mut pages = Vec::with_capacity(4);
        let mut lines = Vec::with_capacity(50);

        for (i, line) in txt.lines().enumerate() {
            let text = ansi_parse::AnsiTextParser::new(line)
                .filter_map(|txt| {
                    text_fn.call_checked(TextFnArgs {
                        txt: txt.txt.to_txt(),
                        style: txt.style,
                    })
                })
                .collect();

            lines.push(line_fn(LineFnArgs {
                index: i as u32,
                page_index: lines.len() as u32,
                text,
            }));

            if lines.len() == lines_per_page {
                let lines = mem::replace(&mut lines, Vec::with_capacity(50));
                pages.push(page_fn(PageFnArgs {
                    index: pages.len() as u32,
                    lines: lines.into(),
                }));
            }
        }

        if !lines.is_empty() {
            pages.push(page_fn(PageFnArgs {
                index: pages.len() as u32,
                lines: lines.into(),
            }));
        }

        panel_fn(PanelFnArgs { pages: pages.into() })
    })
}

/// Implements the ANSI parsing and view generation, configured by contextual properties.
pub fn ansi_node(txt: impl IntoVar<Txt>) -> impl UiNode {
    let txt = txt.into_var();
    match_node(NilUiNode.boxed(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_var(&txt)
                .sub_var(&TEXT_FN_VAR)
                .sub_var(&LINE_FN_VAR)
                .sub_var(&PAGE_FN_VAR)
                .sub_var(&PANEL_FN_VAR)
                .sub_var(&LINES_PER_PAGE_VAR)
                .sub_var(&BLINK_INTERVAL_VAR);

            *c.child() = generate_ansi(&txt);
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.child() = NilUiNode.boxed();
        }
        UiNodeOp::Update { .. } => {
            use ansi_fn::*;

            if txt.is_new()
                || TEXT_FN_VAR.is_new()
                || LINE_FN_VAR.is_new()
                || PAGE_FN_VAR.is_new()
                || PANEL_FN_VAR.is_new()
                || LINES_PER_PAGE_VAR.is_new()
                || BLINK_INTERVAL_VAR.is_new()
            {
                c.child().deinit();
                *c.child() = generate_ansi(&txt);
                c.child().init();
                WIDGET.update_info().layout().render();
            }
        }
        _ => {}
    })
}

static X_TERM_256: [(u8, u8, u8); 256] = [
    (0, 0, 0),
    (128, 0, 0),
    (0, 128, 0),
    (128, 128, 0),
    (0, 0, 128),
    (128, 0, 128),
    (0, 128, 128),
    (192, 192, 192),
    (128, 128, 128),
    (255, 0, 0),
    (0, 255, 0),
    (255, 255, 0),
    (0, 0, 255),
    (255, 0, 255),
    (0, 255, 255),
    (255, 255, 255),
    (0, 0, 0),
    (0, 0, 95),
    (0, 0, 135),
    (0, 0, 175),
    (0, 0, 215),
    (0, 0, 255),
    (0, 95, 0),
    (0, 95, 95),
    (0, 95, 135),
    (0, 95, 175),
    (0, 95, 215),
    (0, 95, 255),
    (0, 135, 0),
    (0, 135, 95),
    (0, 135, 135),
    (0, 135, 175),
    (0, 135, 215),
    (0, 135, 255),
    (0, 175, 0),
    (0, 175, 95),
    (0, 175, 135),
    (0, 175, 175),
    (0, 175, 215),
    (0, 175, 255),
    (0, 215, 0),
    (0, 215, 95),
    (0, 215, 135),
    (0, 215, 175),
    (0, 215, 215),
    (0, 215, 255),
    (0, 255, 0),
    (0, 255, 95),
    (0, 255, 135),
    (0, 255, 175),
    (0, 255, 215),
    (0, 255, 255),
    (95, 0, 0),
    (95, 0, 95),
    (95, 0, 135),
    (95, 0, 175),
    (95, 0, 215),
    (95, 0, 255),
    (95, 95, 0),
    (95, 95, 95),
    (95, 95, 135),
    (95, 95, 175),
    (95, 95, 215),
    (95, 95, 255),
    (95, 135, 0),
    (95, 135, 95),
    (95, 135, 135),
    (95, 135, 175),
    (95, 135, 215),
    (95, 135, 255),
    (95, 175, 0),
    (95, 175, 95),
    (95, 175, 135),
    (95, 175, 175),
    (95, 175, 215),
    (95, 175, 255),
    (95, 215, 0),
    (95, 215, 95),
    (95, 215, 135),
    (95, 215, 175),
    (95, 215, 215),
    (95, 215, 255),
    (95, 255, 0),
    (95, 255, 95),
    (95, 255, 135),
    (95, 255, 175),
    (95, 255, 215),
    (95, 255, 255),
    (135, 0, 0),
    (135, 0, 95),
    (135, 0, 135),
    (135, 0, 175),
    (135, 0, 215),
    (135, 0, 255),
    (135, 95, 0),
    (135, 95, 95),
    (135, 95, 135),
    (135, 95, 175),
    (135, 95, 215),
    (135, 95, 255),
    (135, 135, 0),
    (135, 135, 95),
    (135, 135, 135),
    (135, 135, 175),
    (135, 135, 215),
    (135, 135, 255),
    (135, 175, 0),
    (135, 175, 95),
    (135, 175, 135),
    (135, 175, 175),
    (135, 175, 215),
    (135, 175, 255),
    (135, 215, 0),
    (135, 215, 95),
    (135, 215, 135),
    (135, 215, 175),
    (135, 215, 215),
    (135, 215, 255),
    (135, 255, 0),
    (135, 255, 95),
    (135, 255, 135),
    (135, 255, 175),
    (135, 255, 215),
    (135, 255, 255),
    (175, 0, 0),
    (175, 0, 95),
    (175, 0, 135),
    (175, 0, 175),
    (175, 0, 215),
    (175, 0, 255),
    (175, 95, 0),
    (175, 95, 95),
    (175, 95, 135),
    (175, 95, 175),
    (175, 95, 215),
    (175, 95, 255),
    (175, 135, 0),
    (175, 135, 95),
    (175, 135, 135),
    (175, 135, 175),
    (175, 135, 215),
    (175, 135, 255),
    (175, 175, 0),
    (175, 175, 95),
    (175, 175, 135),
    (175, 175, 175),
    (175, 175, 215),
    (175, 175, 255),
    (175, 215, 0),
    (175, 215, 95),
    (175, 215, 135),
    (175, 215, 175),
    (175, 215, 215),
    (175, 215, 255),
    (175, 255, 0),
    (175, 255, 95),
    (175, 255, 135),
    (175, 255, 175),
    (175, 255, 215),
    (175, 255, 255),
    (215, 0, 0),
    (215, 0, 95),
    (215, 0, 135),
    (215, 0, 175),
    (215, 0, 215),
    (215, 0, 255),
    (215, 95, 0),
    (215, 95, 95),
    (215, 95, 135),
    (215, 95, 175),
    (215, 95, 215),
    (215, 95, 255),
    (215, 135, 0),
    (215, 135, 95),
    (215, 135, 135),
    (215, 135, 175),
    (215, 135, 215),
    (215, 135, 255),
    (215, 175, 0),
    (215, 175, 95),
    (215, 175, 135),
    (215, 175, 175),
    (215, 175, 215),
    (215, 175, 255),
    (215, 215, 0),
    (215, 215, 95),
    (215, 215, 135),
    (215, 215, 175),
    (215, 215, 215),
    (215, 215, 255),
    (215, 255, 0),
    (215, 255, 95),
    (215, 255, 135),
    (215, 255, 175),
    (215, 255, 215),
    (215, 255, 255),
    (255, 0, 0),
    (255, 0, 95),
    (255, 0, 135),
    (255, 0, 175),
    (255, 0, 215),
    (255, 0, 255),
    (255, 95, 0),
    (255, 95, 95),
    (255, 95, 135),
    (255, 95, 175),
    (255, 95, 215),
    (255, 95, 255),
    (255, 135, 0),
    (255, 135, 95),
    (255, 135, 135),
    (255, 135, 175),
    (255, 135, 215),
    (255, 135, 255),
    (255, 175, 0),
    (255, 175, 95),
    (255, 175, 135),
    (255, 175, 175),
    (255, 175, 215),
    (255, 175, 255),
    (255, 215, 0),
    (255, 215, 95),
    (255, 215, 135),
    (255, 215, 175),
    (255, 215, 215),
    (255, 215, 255),
    (255, 255, 0),
    (255, 255, 95),
    (255, 255, 135),
    (255, 255, 175),
    (255, 255, 215),
    (255, 255, 255),
    (8, 8, 8),
    (18, 18, 18),
    (28, 28, 28),
    (38, 38, 38),
    (48, 48, 48),
    (58, 58, 58),
    (68, 68, 68),
    (78, 78, 78),
    (88, 88, 88),
    (98, 98, 98),
    (108, 108, 108),
    (118, 118, 118),
    (128, 128, 128),
    (138, 138, 138),
    (148, 148, 148),
    (158, 158, 158),
    (168, 168, 168),
    (178, 178, 178),
    (188, 188, 188),
    (198, 198, 198),
    (208, 208, 208),
    (218, 218, 218),
    (228, 228, 228),
    (238, 238, 238),
];
