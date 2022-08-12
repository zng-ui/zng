use crate::prelude::new_widget::*;

pub mod nodes;
pub mod properties;

/// A configured text run.
///
/// # Examples
///
/// ```
/// use zero_ui::prelude::text;
///
/// let hello_txt = text! {
///     font_family = "Arial";
///     font_size = 18;
///     text = "Hello!";
/// };
/// ```
/// # As Function
///
/// If you don't need to configure the text, you can just use the function [`text`](fn@text).
#[widget($crate::widgets::text)]
pub mod text {
    use crate::prelude::new_widget::*;

    pub use super::{nodes, properties};

    properties! {
        /// The [`Text`](crate::core::types::Text) value.
        ///
        /// Set to an empty string (`""`) by default.
        text(impl IntoVar<Text>) = "";

        /// Spacing in between the text and background edges or border.
        ///
        /// Set to `0` by default.
        padding = 0;

        /// The text font. If not set inherits the `font_family` from the parent widget.
        properties::font_family;
        /// The font style. If not set inherits the `font_style` from the parent widget.
        properties::font_style;
        /// The font weight. If not set inherits the `font_weight` from the parent widget.
        properties::font_weight;
        /// The font stretch. If not set inherits the `font_stretch` from the parent widget.
        properties::font_stretch;
        /// The font size. If not set inherits the `font_size` from the parent widget.
        properties::font_size;
        /// The text color. If not set inherits the `text_color` from the parent widget.
        properties::text_color as color;

        /// The text alignment.
        properties::text_align;

        /// Extra spacing added in between text letters. If not set inherits the `letter_spacing` from the parent widget.
        ///
        /// Letter spacing is computed using the font data, this unit represents
        /// extra space added to the computed spacing.
        ///
        /// A "letter" is a character glyph cluster, e.g.: `a`, `â`, `1`, `-`, `漢`.
        ///
        /// The [`Default`] value signals that letter spacing can be tweaked when text *justification* is enabled, all other
        /// values disable automatic adjustments for justification inside words.
        ///
        /// Relative values are computed from the length of the space `' '` character.
        ///
        /// [`Default`]: Length::Default
        properties::letter_spacing;

        /// Extra spacing added to the Unicode `U+0020 SPACE` character. If not set inherits the `letter_spacing` from the parent widget.
        ///
        /// Word spacing is done using the space character "advance" as defined in the font,
        /// this unit represents extra spacing added to that default spacing.
        ///
        /// A "word" is the sequence of characters in-between space characters. This extra
        /// spacing is applied per space character not per word, if there are three spaces between words
        /// the extra spacing is applied thrice. Usually the number of spaces between words is collapsed to one,
        /// see [`WhiteSpace`](crate::text::WhiteSpace).
        ///
        /// The [`Default`] value signals that word spacing can be tweaked when text *justification* is enabled, all other
        /// values disable automatic adjustments for justification. Relative values are computed from the length of the space `' '` character,
        /// so a word spacing of `100.pct()` visually adds *another* space in between words.
        ///
        /// [`Default`]: Length::Default
        properties::word_spacing;

        /// Height of each text line. If not set inherits the `line_height` from the parent widget.
        ///
        /// The [`Default`] value is computed from the font metrics, `ascent - descent + line_gap`, this is
        /// usually similar to `1.2.em()`. Relative values are computed from the default value, so `200.pct()` is double
        /// the default line height.
        ///
        /// The text is vertically centralized inside the height.
        ///
        /// [`Default`]: Length::Default
        properties::line_height;
        /// Extra spacing in-between text lines. If not set inherits the `line_spacing` from the parent widget.
        ///
        /// The [`Default`] value is zero. Relative values are calculated from the [`LineHeight`], so `50.pct()` is half
        /// the computed line height. If the text only has one line this property is not used.
        ///
        /// [`Default`]: Length::Default
        properties::line_spacing;

        /// Draw lines *above* each text line.
        properties::overline;
        /// Custom [`overline`](#wp-overline) color, if not set
        /// the [`color`](#wp-color) is used.
        properties::overline_color;

        /// Draw lines across each text line.
        properties::strikethrough;
        /// Custom [`strikethrough`](#wp-strikethrough) color, if not set
        /// the [`color`](#wp-color) is used.
        properties::strikethrough_color;

        /// Draw lines *under* each text line.
        properties::underline;
        /// Custom [`underline`](#wp-underline) color, if not set
        /// the [`color`](#wp-color) is used.
        properties::underline_color;
        /// Defines what segments of each text line are skipped when tracing the [`underline`](#wp-underline).
        ///
        /// By default skips glyphs that intercept the underline.
        properties::underline_skip;
        /// Defines what font line gets traced by the underline.
        ///
        /// By default uses the font configuration, but it usually crosses over glyph *descents* causing skips on
        /// the line, you can set this [`UnderlinePosition::Descent`] to fully clear all glyph *descents*.
        properties::underline_position;

        /// Enable text selection, copy, caret and input; and makes the widget focusable.
        ///
        /// If the `text` variable is read-only, this only enables text selection, if the var is writeable this
        /// enables text input and modifies the variable.
        properties::text_editable as editable;
    }

    fn new_child() -> impl UiNode {
        let child = nodes::render_text();
        let child = nodes::render_overlines(child);
        let child = nodes::render_strikethroughs(child);
        nodes::render_underlines(child)
    }

    fn new_fill(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
        nodes::layout_text(child, padding)
    }

    fn new_event(child: impl UiNode, text: impl IntoVar<Text>) -> impl UiNode {
        nodes::resolve_text(child, text)
    }
}

/// Simple text run.
///
/// # Configure
///
/// Text spans can be configured by setting [`font_family`], [`font_size`] and other properties in parent widgets.
///
/// # Examples
///
/// ```
/// # fn main() -> () {
/// use zero_ui::widgets::{container, text, text::properties::{font_family, font_size}};
///
/// let hello_txt = container! {
///     font_family = "Arial";
///     font_size = 18;
///     content = text("Hello!");
/// };
/// # }
/// ```
///
/// # `text!`
///
/// There is a specific widget for creating configured text runs: [`text!`].
///
/// [`font_family`]: fn@crate::widgets::text::properties::font_family
/// [`font_size`]: fn@crate::widgets::text::properties::font_size
/// [`text_color`]: fn@crate::widgets::text::properties::text_color
/// [`text!`]: mod@text
pub fn text(text: impl IntoVar<Text> + 'static) -> impl Widget {
    // TODO remove 'static when rust issue #42940 is fixed.
    text! { text; }
}

#[widget($crate::widgets::text_wgt::strong)]
mod strong {
    use super::*;

    properties! {
        text(impl IntoVar<Text>);
    }

    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        let child = nodes::render_text();
        let child = nodes::layout_text(child, 0);
        let child = nodes::resolve_text(child, text);
        font_weight(child, FontWeight::BOLD)
    }
}

/// A simple text run with **bold** font weight.
///
/// # Configure
///
/// Apart from the font weight this widget can be configured with contextual properties like [`text`](function@text).
pub fn strong(text: impl IntoVar<Text> + 'static) -> impl Widget {
    strong! { text; }
}

#[widget($crate::widgets::text_wgt::em)]
mod em {
    use super::*;

    properties! {
        text(impl IntoVar<Text>);
    }

    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        let child = nodes::render_text();
        let child = nodes::layout_text(child, 0);
        let child = nodes::resolve_text(child, text);
        font_style(child, FontStyle::Italic)
    }
}

/// A simple text run with *italic* font style.
///
/// # Configure
///
/// Apart from the font style this widget can be configured with contextual properties like [`text`](function@text).
pub fn em(text: impl IntoVar<Text> + 'static) -> impl Widget {
    em! { text; }
}

/// Text box widget.
#[widget($crate::widgets::text_input)]
pub mod text_input {
    use super::*;

    inherit!(super::text);

    properties! {
        editable = true;

        /// Text input background color.
        background_color = theme::BackgroundColorVar;

        /// Text input border.
        border = {
            widths: theme::BorderWidthsVar,
            sides: theme::BorderSidesVar,
        };

        /// Text input corner radius.
        corner_radius = theme::CornerRadiusVar;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the text input is pressed.
        capture_mouse = true;

        /// Content padding.
        padding = theme::PaddingVar;

        /// Text input cursor.
        cursor = theme::CursorIconVar;

        /// Enables keyboard focusing in the widget.
        focusable = true;

        /// When the pointer device is over this text input.
        when self.is_cap_hovered {
            background_color = theme::hovered::BackgroundColorVar;
            border = {
                widths: theme::BorderWidthsVar,
                sides: theme::hovered::BorderSidesVar,
            };
            text_color = theme::hovered::TextColorVar;
        }

        /// When this text input has keyboard input.
        when self.is_focused {
            background_color = theme::focused::BackgroundColorVar;
            border = {
                widths: theme::BorderWidthsVar,
                sides: theme::focused::BorderSidesVar,
            };
            text_color = theme::focused::TextColorVar;
        }

        /// When the text input is disabled.
        when self.is_disabled {
            background_color = theme::disabled::BackgroundColorVar;
            border = {
                widths: theme::BorderWidthsVar,
                sides: theme::disabled::BorderSidesVar,
            };
            text_color = theme::disabled::TextColorVar;
            cursor = theme::disabled::CursorIconVar;
        }
    }

    /// Context variables and properties that affect the text input appearance from parent widgets.
    pub mod theme {
        use super::*;

        context_var! {
            /// Text input background color.
            ///
            /// Use the [`text_input::theme::background_color`] property to set.
            ///
            /// [`text_input::theme::background_color`]: fn@background_color
            pub struct BackgroundColorVar: Rgba = rgb(0.1, 0.1, 0.1);

            /// Text input border widths.
            ///
            /// Use the [`text_input::theme::border`] property to set.
            ///`text_input
            /// [`text_input::theme::border`]: fn@border
            pub struct BorderWidthsVar: SideOffsets = SideOffsets::new_all(1);
            /// Text input border sides.
            ///
            /// Use the [`text_input::theme::border`] property to set.
            ///
            /// [`text_input::theme::border`]: fn@border
            pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.3, 0.3, 0.3));
            /// Text input corner radius.
            ///
            /// Use the [`text_input::theme::corner_radius`] property to set.
            ///
            /// [`text_input::theme::corner_radius`]: fn@corner_radius
            pub struct CornerRadiusVar: CornerRadius = CornerRadius::new_all(2);

            /// Text input padding.
            ///
            /// Use the [`text_input::theme::padding`] property to set.
            ///
            /// [`text_input::theme::border`]: fn@border
            pub struct PaddingVar: SideOffsets = SideOffsets::new(7, 15, 7, 15);

            /// Text input cursor icon.
            ///
            /// Use the [`text_input::theme::cursor`] property to set.
            ///
            /// Default is [`CursorIcon::Default`].
            pub struct CursorIconVar: Option<CursorIcon> = Some(CursorIcon::Text);
        }

        /// Sets the [`BackgroundColorVar`] that affects all buttons inside the widget.
        #[property(context, default(BackgroundColorVar))]
        pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
            with_context_var(child, BackgroundColorVar, color)
        }

        /// Sets the [`BorderWidthsVar`], [`BorderSidesVar`] that affects all buttons inside the widget.
        #[property(context, default(BorderWidthsVar, BorderSidesVar))]
        pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, sides: impl IntoVar<BorderSides>) -> impl UiNode {
            let child = with_context_var(child, BorderWidthsVar, widths);
            with_context_var(child, BorderSidesVar, sides)
        }

        /// Sets the [`CornerRadiusVar`] that affects all buttons inside the widget.
        #[property(context, default(CornerRadiusVar))]
        pub fn corner_radius(child: impl UiNode, radius: impl IntoVar<CornerRadius>) -> impl UiNode {
            with_context_var(child, CornerRadiusVar, radius)
        }

        /// Sets the [`PaddingVar`] that affects all buttons inside the widget.
        #[property(context, default(PaddingVar))]
        pub fn padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
            with_context_var(child, PaddingVar, padding)
        }

        /// Sets the [`CursorIconVar`] that affects all buttons inside the widget.
        #[property(context, default(CursorIconVar))]
        pub fn cursor(child: impl UiNode, align: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
            with_context_var(child, CursorIconVar, align)
        }

        /// Pointer hovered values.
        pub mod hovered {
            use super::*;

            context_var! {
                /// Hovered text input background color.
                ///
                /// Use the [`text_input::theme::hovered::background_color`] property to set.
                ///
                /// [`text_input::theme::hovered::background_color`]: fn@background_color
                pub struct BackgroundColorVar: Rgba = rgb(0.1, 0.1, 0.1);

                /// Hovered text input border sides.
                ///
                /// Use the [`text_input::theme::hovered::border_sides`] property to set.
                ///
                /// [`text_input::theme::hovered::border_sides`]: fn@border_sides
                pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.4, 0.4, 0.4));

                /// Hovered text input text color.
                ///
                /// Use the [`text_input::theme::hovered::text_color`] property to set.
                ///
                /// [`text_input::theme::hovered::text_color`]: fn@text_color
                pub struct TextColorVar: Rgba = colors::WHITE;
            }

            /// Sets the hovered [`BackgroundColorVar`] that affects all buttons inside the widget.
            #[property(context, default(BackgroundColorVar))]
            pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, BackgroundColorVar, color)
            }

            /// Sets the hovered [`BorderSidesVar`] that affects all buttons inside the widget.
            #[property(context, default(BorderSidesVar))]
            pub fn border_sides(child: impl UiNode, sides: impl IntoVar<BorderSides>) -> impl UiNode {
                with_context_var(child, BorderSidesVar, sides)
            }

            /// Sets the hovered [`TextColorVar`] that affects all texts inside buttons inside the widget.
            #[property(context, default(TextColorVar))]
            pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, TextColorVar, color)
            }
        }

        /// Focused values.
        pub mod focused {
            use super::*;

            context_var! {
                /// Hovered text input background color.
                ///
                /// Use the [`text_input::theme::hovered::background_color`] property to set.
                ///
                /// [`text_input::theme::hovered::background_color`]: fn@background_color
                pub struct BackgroundColorVar: Rgba = rgb(0.1, 0.1, 0.1);

                /// Hovered text input border sides.
                ///
                /// Use the [`text_input::theme::hovered::border_sides`] property to set.
                ///
                /// [`text_input::theme::hovered::border_sides`]: fn@border_sides
                pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.6, 0.6, 0.6));

                /// Hovered text input text color.
                ///
                /// Use the [`text_input::theme::hovered::text_color`] property to set.
                ///
                /// [`text_input::theme::hovered::text_color`]: fn@text_color
                pub struct TextColorVar: Rgba = colors::WHITE;
            }

            /// Sets the focused [`BackgroundColorVar`] that affects all buttons inside the widget.
            #[property(context, default(BackgroundColorVar))]
            pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, BackgroundColorVar, color)
            }

            /// Sets the focused [`BorderSidesVar`] that affects all buttons inside the widget.
            #[property(context, default(BorderSidesVar))]
            pub fn border_sides(child: impl UiNode, sides: impl IntoVar<BorderSides>) -> impl UiNode {
                with_context_var(child, BorderSidesVar, sides)
            }

            /// Sets the focused [`TextColorVar`] that affects all texts inside buttons inside the widget.
            #[property(context, default(TextColorVar))]
            pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, TextColorVar, color)
            }
        }

        /// Text input disabled values.
        pub mod disabled {
            use super::*;

            context_var! {
                /// Disabled text input background color.
                ///
                /// Use the [`text_input::theme::disabled::background_color`] property to set.
                ///
                /// [`text_input::theme::disabled::background_color`]: fn@background_color
                pub struct BackgroundColorVar: Rgba = rgb(0.1, 0.1, 0.1);
                /// Disabled text input border sides.
                ///
                /// Use the [`text_input::theme::disabled::border`] property to set.
                ///
                /// [`text_input::theme::disabled::border`]: fn@border
                pub struct BorderSidesVar: BorderSides = BorderSides::solid(rgb(0.2, 0.2, 0.2));

                /// Disabled text input text color.
                ///
                /// Use the [`text_input::theme::disabled::text_color`] property to set.
                ///
                /// [`text_input::theme::disabled::text_color`]: fn@text_color
                pub struct TextColorVar: Rgba = colors::WHITE.darken(40.pct());

                /// Disabled text input cursor icon.
                ///
                /// Use the [`text_input::theme::disabled::cursor`] property to set.
                ///
                /// Default is [`CursorIcon::NotAllowed`], meaning the parent cursor is used.
                pub struct CursorIconVar: Option<CursorIcon> = Some(CursorIcon::NotAllowed);
            }

            /// Sets the disabled [`BackgroundColorVar`] that affects all buttons inside the widget.
            #[property(context, default(BackgroundColorVar))]
            pub fn background_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, BackgroundColorVar, color)
            }

            /// Sets the disabled [`BorderSidesVar`] that affects all buttons inside the widget.
            #[property(context, default(BorderSidesVar))]
            pub fn border_sides(child: impl UiNode, sides: impl IntoVar<BorderSides>) -> impl UiNode {
                with_context_var(child, BorderSidesVar, sides)
            }

            /// Sets the disabled [`TextColorVar`] that affects all texts inside buttons inside the widget.
            #[property(context, default(TextColorVar))]
            pub fn text_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
                with_context_var(child, TextColorVar, color)
            }

            /// Sets the disabled [`CursorIconVar`] that affects all buttons inside the widget.
            #[property(context, default(CursorIconVar))]
            pub fn cursor(child: impl UiNode, align: impl IntoVar<Option<CursorIcon>>) -> impl UiNode {
                with_context_var(child, CursorIconVar, align)
            }
        }
    }
}
