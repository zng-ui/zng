use zero_ui::prelude::new_widget::*;

/// Render text styled using ANSI scale sequences.
///
/// Supports color, weight and italic.
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

        font_family = FontName::monospace();
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
            for code in esc_codes.split(';') {
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
                    _ => (),
                }
            }
        }
    }
}

mod ansi_view {

    use ansi_text::{AnsiColor, AnsiStyle, AnsiWeight};
    use zero_ui_core::widget_instance::UiNodeVec;

    use super::*;

    /// Arguments for a view generator for an ANSI styled text fragment.
    ///
    /// See [`ANSI_TEXT_VIEW_VAR`] for more details.
    pub struct AnsiTextViewArgs {
        /// The text.
        pub txt: Text,
        /// The ANSI style.
        pub style: AnsiStyle,
    }

    /// Arguments for a view generator for a text line.
    ///
    /// See [`ANSI_LINE_VIEW_VAR`] for more details.
    pub struct AnsiLineViewArgs {
        /// Line number (first line is `1`).
        pub number: u32,
        /// Text segment widgets, generated by [`ANSI_TEXT_VIEW_VAR`].
        pub text: UiNodeVec,
    }

    /// Arguments for a view generator for a stack of lines.
    ///
    /// See [`ANSI_PANEL_VIEW_VAR`] for more details.
    pub struct AnsiPanelViewArgs {
        /// Line widgets, generated by [`ANSI_LINE_VIEW_VAR`].
        pub lines: UiNodeVec,
    }

    context_var! {
        /// View generator for [`AnsiTextViewArgs`].
        ///
        /// The returned view is inlined in the [`ANSI_LINE_VIEW_VAR`]. The default view is [`default_text_view`].
        pub static ANSI_TEXT_VIEW_VAR: ViewGenerator<AnsiTextViewArgs> = view_generator!(|_, args: AnsiTextViewArgs|  {
            default_text_view(args)
        });

        /// View generator for [`AnsiLineViewArgs`].
        ///
        /// The returned view is stacked in the [`ANSI_PANEL_VIEW_VAR`]. The default view is [`default_line_view`].
        pub static ANSI_LINE_VIEW_VAR: ViewGenerator<AnsiLineViewArgs> = view_generator!(|_, args: AnsiLineViewArgs| {
            default_line_view(args)
        });

        /// View generator for [`AnsiPanelViewArgs`].
        ///
        /// The returned view is the child of the [`ansi_text!`] widget. The default view is [`default_panel_view`].
        ///
        /// [`ansi_text!`]: mod@super::ansi_text
        pub static ANSI_PANEL_VIEW_VAR: ViewGenerator<AnsiPanelViewArgs> = view_generator!(|_, args: AnsiPanelViewArgs| {
            default_panel_view(args)
        });

        /// If the ANSI blink animation is generated when the style requests it.
        pub static ANSI_BLINK_ENABLED_VAR: bool = false;
    }

    /// Default [`ANSI_TEXT_VIEW_VAR`].
    ///
    /// This view is configured by contextual variables like [`ANSI_BLINK_ENABLED_VAR`] and all text variables that are
    /// not overridden by the ANSI style, like the font.
    ///
    /// Returns a `text!` with the text and style.
    pub fn default_text_view(args: AnsiTextViewArgs) -> impl UiNode {
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
        if args.style.blink && !args.style.hidden && ANSI_BLINK_ENABLED_VAR.get() {
            let o = var(1.fct());
            todo!("impl var repeat animation, use it here and keyboard caret animation");

            builder.push_property(
                Importance::INSTANCE,
                property_args! {
                    opacity = o;
                },
            );
        }

        crate::widgets::text::build(builder)
    }

    /// Default [`ANSI_LINE_VIEW_VAR`].
    ///
    /// Returns a `wrap!` for the text.
    pub fn default_line_view(args: AnsiLineViewArgs) -> impl UiNode {
        crate::widgets::layouts::wrap! {
            children = args.text;
        }
    }

    /// Default [`ANSI_PANEL_VIEW_VAR`].
    ///
    /// Returns a `v_stack!`.
    pub fn default_panel_view(args: AnsiPanelViewArgs) -> impl UiNode {
        crate::widgets::layouts::v_stack! {
            children = args.lines;
        }
    }

    /// If the ANSI blink animation is generated when the style requests it.
    ///
    /// Sets the [`ANSI_BLINK_ENABLED_VAR`].
    #[property(CONTEXT, default(ANSI_BLINK_ENABLED_VAR))]
    pub fn blink_enabled(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
        with_context_var(child, ANSI_BLINK_ENABLED_VAR, enabled)
    }

    /// View generator that converts [`AnsiTextViewArgs`] to widgets.
    ///
    /// Sets the [`ANSI_TEXT_VIEW_VAR`].
    #[property(CONTEXT, default(ANSI_TEXT_VIEW_VAR))]
    pub fn ansi_text_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<AnsiTextViewArgs>>) -> impl UiNode {
        with_context_var(child, ANSI_TEXT_VIEW_VAR, view)
    }

    /// View generator that converts [`AnsiLineViewArgs`] to widgets.
    ///
    /// Sets the [`ANSI_LINE_VIEW_VAR`].
    #[property(CONTEXT, default(ANSI_LINE_VIEW_VAR))]
    pub fn ansi_line_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<AnsiLineViewArgs>>) -> impl UiNode {
        with_context_var(child, ANSI_LINE_VIEW_VAR, view)
    }

    /// View generator that converts [`AnsiPanelViewArgs`] to widgets.
    ///
    /// Sets the [`ANSI_PANEL_VIEW_VAR`].
    #[property(CONTEXT, default(ANSI_PANEL_VIEW_VAR))]
    pub fn ansi_panel_view(child: impl UiNode, view: impl IntoVar<ViewGenerator<AnsiPanelViewArgs>>) -> impl UiNode {
        with_context_var(child, ANSI_PANEL_VIEW_VAR, view)
    }
}

/// Implements the ANSI rendering, configured by contextual properties.
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
            self.child = FillUiNode.boxed();
        }

        #[UiNode]
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            if self.txt.is_new(ctx) {
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

            self.child = self.txt.with(|txt| {
                let text_view = ANSI_TEXT_VIEW_VAR.get();
                let line_view = ANSI_LINE_VIEW_VAR.get();
                let panel_view = ANSI_PANEL_VIEW_VAR.get();

                // generate lines
                let lines = txt
                    .lines()
                    .enumerate()
                    .map(|(i, line)| {
                        // generate segments
                        let text = ansi_parse::AnsiTextParser::new(line)
                            .map(|txt| {
                                text_view
                                    .generate(
                                        ctx,
                                        AnsiTextViewArgs {
                                            txt: txt.txt.to_text(),
                                            style: txt.style,
                                        },
                                    )
                                    .boxed()
                            })
                            .collect();

                        line_view
                            .generate(
                                ctx,
                                AnsiLineViewArgs {
                                    number: (i + 1) as u32,
                                    text,
                                },
                            )
                            .boxed()
                    })
                    .collect();

                // generate panel
                panel_view.generate(ctx, AnsiPanelViewArgs { lines }).boxed()
            });
        }
    }
    AnsiNode {
        child: FillUiNode.boxed(),
        txt: txt.into_var(),
    }
}
