use zero_ui::prelude::new_widget::*;

/// Render text styled using ANSI scale sequences.
///
/// Supports color, weight, italic and more, see [`AnsiStyle`] for the full style supported.
/// 
/// [`AnsiStyle`]: ansi_text::AnsiStyle
#[widget($crate::widgets::ansi_text)]
pub mod ansi_text {
    use super::*;

    inherit!(widget_base::base);

    #[doc(inline)]
    pub use ansi_parse::*;

    #[doc(inline)]
    pub use ansi_view::*;

    #[doc(inline)]
    pub use super::ansi_node;

    #[doc(no_inline)]
    pub use crate::widgets::text::{font_family, font_size, tab_length};

    properties! {
        /// ANSI text.
        pub txt(impl IntoVar<Text>) = "";

        font_family = ["JetBrains Mono", "Consolas", "monospace"];
    }

    fn include(wgt: &mut WidgetBuilder) {
        wgt.push_build_action(|wgt| {
            let txt = wgt.capture_var_or_default(property_id!(self::txt));
            let child = ansi_node(txt);
            wgt.set_child(child.boxed());
        });
    }
}

mod ansi_parse {
    use super::*;

    /// Represents a segment of ANSI styled text that shares the same style.
    #[derive(Debug)]
    pub struct AnsiText<'a> {
        /// Text run.
        pub txt: &'a str,
        /// Text style.
        pub style: AnsiStyle,
    }

    /// Represents the ANSI style of a text run.
    ///
    /// See [`AnsiText`] for more details.
    #[derive(Debug, Clone)]
    pub struct AnsiStyle {
        /// Background color.
        pub background_color: AnsiColor,
        /// Foreground color.
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
                },
                AnsiColor::TrueColor(r, g, b) => rgb(r, g, b),
            }
        }
    }

    /// Font weight defined by ANSI escape codes.
    ///
    /// See [`AnsiStyle`] for more details.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// This is the pull style parser used internally by the [`ansi_text!`] widget.
    /// 
    /// [`ansi_text!`]: mod@crate::widgets::ansi_text
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
        type Item = AnsiText<'a>;

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
                    return Some(AnsiText {
                        txt,
                        style: self.style.clone(),
                    });
                } else {
                    return Some(AnsiText {
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

mod ansi_view {

    use std::time::Duration;

    use ansi_text::{AnsiColor, AnsiStyle, AnsiWeight};
    use zero_ui_core::widget_instance::UiNodeVec;

    use super::*;

    /// Arguments for a view generator for an ANSI styled text fragment.
    ///
    /// See [`TEXT_VIEW_VAR`] for more details.
    pub struct TextViewArgs {
        /// The text.
        pub txt: Text,
        /// The ANSI style.
        pub style: AnsiStyle,
    }

    /// Arguments for a view generator for a text line.
    ///
    /// See [`LINE_VIEW_VAR`] for more details.
    pub struct LineViewArgs {
        /// Zero-counted global index of this line.
        pub index: u32,
        /// Zero-counted index of this line in the parent page.
        pub page_index: u32,
        /// Text segment widgets, generated by [`TEXT_VIEW_VAR`].
        pub text: UiNodeVec,
    }

    /// Arguments for a view generator for a stack of lines.
    ///
    /// See [`PAGE_VIEW_VAR`] for more details.
    pub struct PageViewArgs {
        /// Zero-counted index of this page.
        pub index: u32,

        /// Line widgets, generated by [`LINE_VIEW_VAR`].
        pub lines: UiNodeVec,
    }

    /// Arguments for a view generator for a stack of pages.
    ///
    /// See [`PANEL_VIEW_VAR`] for more details.
    pub struct PanelViewArgs {
        /// Page widgets, generated by [`PAGE_VIEW_VAR`].
        pub pages: UiNodeVec,
    }

    context_var! {
        /// View generator for [`TextViewArgs`].
        ///
        /// The returned views are layout by the [`LINE_VIEW_VAR`]. The default view is [`default_text_view`].
        pub static TEXT_VIEW_VAR: ViewGenerator<TextViewArgs> = view_generator!(|ctx, args: TextViewArgs|  {
            default_text_view(ctx.vars, args)
        });

        /// View generator for [`LineViewArgs`].
        ///
        /// The returned views are layout by the [`PAGE_VIEW_VAR`]. The default view is [`default_line_view`].
        pub static LINE_VIEW_VAR: ViewGenerator<LineViewArgs> = view_generator!(|_, args: LineViewArgs| {
            default_line_view(args)
        });

        /// View generator for [`PageViewArgs`].
        ///
        /// The returned views are layout by the [`PANEL_VIEW_VAR`] widget. The default view is [`default_page_view`].
        pub static PAGE_VIEW_VAR: ViewGenerator<PageViewArgs> = view_generator!(|_, args: PageViewArgs| {
            default_page_view(args)
        });

        /// View generator for [`PanelViewArgs`].
        ///
        /// The returned view is the [`ansi_text!`] child. The default is [`default_panel_view`].
        ///
        /// [`ansi_text!`]: mod@super::ansi_text
        pub static PANEL_VIEW_VAR: ViewGenerator<PanelViewArgs> = view_generator!(|_, args: PanelViewArgs| {
            default_panel_view(args)
        });

        /// Duration the ANSI blink animation keeps the text visible for.
        ///
        /// Set to `ZERO` or `MAX` to disable animation.
        pub static BLINK_INTERVAL_VAR: Duration = Duration::ZERO;

        /// Maximum number of lines per [`PAGE_VIEW_VAR`].
        ///
        /// Is `200` by default.
        pub static LINES_PER_PAGE_VAR: u32 = 200;
    }

    /// Default [`TEXT_VIEW_VAR`].
    ///
    /// This view is configured by contextual variables like [`BLINK_INTERVAL_VAR`] and all text variables that are
    /// not overridden by the ANSI style, like the font.
    ///
    /// Returns a `text!` with the text and style.
    pub fn default_text_view(vars: &Vars, args: TextViewArgs) -> impl UiNode {
        use crate::widgets::text as t;

        let mut builder = WidgetBuilder::new(widget_mod!(t));
        t::include(&mut builder);

        builder.push_property(
            Importance::INSTANCE,
            property_args! {
                t::txt = args.txt;
            },
        );

        if args.style.background_color != AnsiColor::Black {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    background_color = args.style.background_color;
                },
            );
        }
        if args.style.color != AnsiColor::White {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::txt_color = args.style.color;
                },
            );
        }

        if args.style.weight != AnsiWeight::Normal {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::font_weight = args.style.weight;
                },
            );
        }
        if args.style.italic {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::font_style = FontStyle::Italic;
                },
            );
        }

        if args.style.underline {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::underline = 1, LineStyle::Solid;
                },
            );
        }
        if args.style.strikethrough {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::strikethrough = 1, LineStyle::Solid;
                },
            );
        }

        if args.style.invert_color {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    invert_color = true;
                },
            );
        }

        if args.style.hidden {
            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    t::visibility = Visibility::Hidden;
                },
            );
        }
        if args.style.blink && !args.style.hidden {
            let opacity = var(1.fct());

            let interval = BLINK_INTERVAL_VAR.get();
            if interval != Duration::ZERO && interval != Duration::MAX {
                opacity.step_oci(vars, 0.fct(), interval, usize::MAX).perm();

                builder.push_property(
                    Importance::INSTANCE,
                    property_args! {
                        opacity;
                    },
                );
            }
        }

        crate::widgets::text::build(builder)
    }

    /// Default [`LINE_VIEW_VAR`].
    ///
    /// Returns a `wrap!` for text with multiple segments, or returns the single segment, or an empty text.
    pub fn default_line_view(mut args: LineViewArgs) -> impl UiNode {
        if args.text.is_empty() {
            crate::widgets::text("").boxed()
        } else if args.text.len() == 1 {
            args.text.remove(0)
        } else {
            crate::widgets::layouts::wrap! {
                children = args.text;
            }
            .boxed()
        }
    }

    /// Default [`PAGE_VIEW_VAR`].
    ///
    /// Returns a `v_stack!` for multiple lines, or return the single line, or a nil node.
    pub fn default_page_view(mut args: PageViewArgs) -> impl UiNode {
        if args.lines.is_empty() {
            NilUiNode.boxed()
        } else if args.lines.len() == 1 {
            args.lines.remove(0)
        } else {
            crate::widgets::layouts::v_stack! {
                children = args.lines;
            }
            .boxed()
        }
    }

    /// Default [`PANEL_VIEW_VAR`].
    ///
    /// Returns a `v_stack!` for multiple pages, or returns the single page, or a nil node.
    pub fn default_panel_view(mut args: PanelViewArgs) -> impl UiNode {
        if args.pages.is_empty() {
            NilUiNode.boxed()
        } else if args.pages.len() == 1 {
            args.pages.remove(0)
        } else {
            crate::widgets::layouts::v_stack! {
                children = args.pages
            }
            .boxed()
        }
    }

    /// ANSI blink animation interval.
    ///
    /// Set to `ZERO` to disable.
    ///
    /// Sets the [`BLINK_INTERVAL_VAR`].
    #[property(CONTEXT, default(BLINK_INTERVAL_VAR))]
    pub fn blink_interval(child: impl UiNode, interval: impl IntoVar<Duration>) -> impl UiNode {
        with_context_var(child, BLINK_INTERVAL_VAR, interval)
    }

    /// View generator that converts [`TextViewArgs`] to widgets.
    ///
    /// Sets the [`TEXT_VIEW_VAR`].
    #[property(CONTEXT, default(TEXT_VIEW_VAR))]
    pub fn text_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<TextViewArgs>>) -> impl UiNode {
        with_context_var(child, TEXT_VIEW_VAR, view)
    }

    /// View generator that converts [`LineViewArgs`] to widgets.
    ///
    /// Sets the [`LINE_VIEW_VAR`].
    #[property(CONTEXT, default(LINE_VIEW_VAR))]
    pub fn line_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<LineViewArgs>>) -> impl UiNode {
        with_context_var(child, LINE_VIEW_VAR, view)
    }

    /// View generator that converts [`PageViewArgs`] to widgets.
    ///
    /// A *page* is a stack of a maximum of [`lines_per_page`], the text is split in pages mostly for performance reasons.
    ///
    /// Sets the [`PAGE_VIEW_VAR`].
    ///
    /// [`lines_per_page`]: fn@lines_per_page
    #[property(CONTEXT, default(PAGE_VIEW_VAR))]
    pub fn page_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<PageViewArgs>>) -> impl UiNode {
        with_context_var(child, PAGE_VIEW_VAR, view)
    }

    /// View generator that converts [``]
    #[property(CONTEXT, default(PANEL_VIEW_VAR))]
    pub fn panel_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<PanelViewArgs>>) -> impl UiNode {
        with_context_var(child, PANEL_VIEW_VAR, view)
    }

    /// Maximum number of lines per page view.
    ///
    /// Sets the [`LINES_PER_PAGE_VAR`].
    #[property(CONTEXT, default(LINES_PER_PAGE_VAR))]
    pub fn lines_per_page(child: impl UiNode, count: impl IntoVar<u32>) -> impl UiNode {
        with_context_var(child, LINES_PER_PAGE_VAR, count)
    }
}

/// Implements the ANSI parsing and view generation, configured by contextual properties.
pub fn ansi_node(txt: impl IntoVar<Text>) -> impl UiNode {
    #[ui_node(struct AnsiNode {
        child: BoxedUiNode,
        #[var] txt: impl Var<Text>,
    })]
    impl AnsiNode {
        #[UiNode]
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.init_handles(ctx);
            self.generate_child(ctx);
            self.child.init(ctx);
        }

        #[UiNode]
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);
            self.child = NilUiNode.boxed();
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            use ansi_view::*;

            if self.txt.is_new(ctx)
                || TEXT_VIEW_VAR.is_new(ctx)
                || LINE_VIEW_VAR.is_new(ctx)
                || PAGE_VIEW_VAR.is_new(ctx)
                || PANEL_VIEW_VAR.is_new(ctx)
                || LINES_PER_PAGE_VAR.is_new(ctx)
                || BLINK_INTERVAL_VAR.is_new(ctx)
            {
                self.child.deinit(ctx);
                self.generate_child(ctx);
                self.child.init(ctx);
                ctx.updates.info_layout_render();
            } else {
                self.child.update(ctx, updates);
            }
        }

        fn generate_child(&mut self, ctx: &mut WidgetContext) {
            use ansi_view::*;
            use std::mem;

            self.child = self.txt.with(|txt| {
                let text_view = TEXT_VIEW_VAR.get();
                let line_view = LINE_VIEW_VAR.get();
                let page_view = PAGE_VIEW_VAR.get();
                let panel_view = PANEL_VIEW_VAR.get();
                let lines_per_page = LINES_PER_PAGE_VAR.get() as usize;

                let mut pages = Vec::with_capacity(4);
                let mut lines = Vec::with_capacity(50);

                for (i, line) in txt.lines().enumerate() {
                    let text = ansi_parse::AnsiTextParser::new(line)
                        .map(|txt| {
                            text_view
                                .generate(
                                    ctx,
                                    TextViewArgs {
                                        txt: txt.txt.to_text(),
                                        style: txt.style,
                                    },
                                )
                                .boxed()
                        })
                        .collect();

                    lines.push(
                        line_view
                            .generate(
                                ctx,
                                LineViewArgs {
                                    index: i as u32,
                                    page_index: lines.len() as u32,
                                    text,
                                },
                            )
                            .boxed(),
                    );

                    if lines.len() == lines_per_page {
                        let lines = mem::replace(&mut lines, Vec::with_capacity(50));
                        pages.push(page_view.generate(
                            ctx,
                            PageViewArgs {
                                index: pages.len() as u32,
                                lines: lines.into(),
                            },
                        ));
                    }
                }

                if !lines.is_empty() {
                    pages.push(page_view.generate(
                        ctx,
                        PageViewArgs {
                            index: pages.len() as u32,
                            lines: lines.into(),
                        },
                    ));
                }

                panel_view.generate(ctx, PanelViewArgs { pages: pages.into() })
            });
        }
    }
    AnsiNode {
        child: NilUiNode.boxed(),
        txt: txt.into_var(),
    }
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
