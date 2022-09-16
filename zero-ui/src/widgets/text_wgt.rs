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
        properties::text_padding as padding;

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
        let child = nodes::render_caret(child);
        let child = nodes::render_overlines(child);
        let child = nodes::render_strikethroughs(child);
        nodes::render_underlines(child)
    }

    fn new_fill(child: impl UiNode) -> impl UiNode {
        nodes::layout_text(child)
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
pub fn text(text: impl IntoVar<Text>) -> impl Widget {
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
        let child = nodes::layout_text(child);
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
        let child = nodes::layout_text(child);
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

    use crate::widgets::themable;

    inherit!(super::text);

    properties! {
        /// Enabled by default.
        editable = true;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the text input is pressed.
        capture_mouse = true;

        /// Enables keyboard focusing in the widget.
        focusable = true;

        /// Theme generator used for the widget.
        ///
        /// Set to [`vis::THEME_VAR`] by default, setting this property directly completely replaces the text input theme,
        /// see [`vis::replace_theme`] and [`vis::extend_theme`] for other ways of modifying the theme.
        theme(impl IntoVar<ThemeGenerator>) = vis::THEME_VAR;
    }

    /// Themable `new`, captures the `id` and `theme` properties.
    pub fn new_dyn(widget: DynWidget, id: impl IntoValue<WidgetId>, theme: impl IntoVar<ThemeGenerator>) -> impl Widget {
        themable::new_dyn(widget, id, theme)
    }

    #[doc(inline)]
    pub use super::text_input_vis as vis;
}

/// Text input theme, visual properties and context vars.
pub mod text_input_vis {
    use super::*;

    context_var! {
        /// Text input theme in a context.
        ///
        /// Is the [`default_theme!`] by default.
        ///
        /// [`default_theme!`]: mod@default_theme
        pub static THEME_VAR: ThemeGenerator = ThemeGenerator::new(|_, _| default_theme!());

        /// Idle background dark and light color.
        pub static BASE_COLORS_VAR: theme::ColorPair = (rgb(0.12, 0.12, 0.12), rgb(0.88, 0.88, 0.88));
    }

    /// Sets the [`BASE_COLORS_VAR`] that is used to compute all background and border colors in the text input theme.
    #[property(context, default(BASE_COLORS_VAR))]
    pub fn base_colors(child: impl UiNode, color: impl IntoVar<theme::ColorPair>) -> impl UiNode {
        with_context_var(child, BASE_COLORS_VAR, color)
    }

    /// Sets the text input theme in a context, the parent theme is fully replaced.
    #[property(context, default(THEME_VAR))]
    pub fn replace_theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, THEME_VAR, theme)
    }

    /// Extends the text input theme in a context, the parent theme is used, properties of the same name set in
    /// `theme` override the parent theme.
    #[property(context, default(ThemeGenerator::nil()))]
    pub fn extend_theme(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        themable::with_theme_extension(child, THEME_VAR, theme)
    }

    /// Default border color.
    pub fn border_color() -> impl Var<Rgba> {
        theme::color_highlight(BASE_COLORS_VAR, 0.20)
    }

    /// Border color hovered.
    pub fn border_color_hovered() -> impl Var<Rgba> {
        theme::color_highlight(BASE_COLORS_VAR, 0.30)
    }

    /// Border color focused.
    pub fn border_color_focused() -> impl Var<Rgba> {
        theme::color_highlight(BASE_COLORS_VAR, 0.40)
    }

    /// Text input default theme.
    #[widget($crate::widgets::text_input::vis::default_theme)]
    pub mod default_theme {
        use super::*;

        inherit!(theme);

        properties! {
            /// Text padding.
            ///
            /// Is `(7, 15)` by default.
            properties::text_padding as padding = (7, 15);

            /// Text cursor.
            cursor = CursorIcon::Text;

            /// Caret color.
            properties::caret_color;

            /// Text input theme base dark and light colors.
            ///
            /// All other text input theme colors are derived from this pair.
            base_colors;

            /// Text input background color.
            background_color = theme::color(BASE_COLORS_VAR);

            /// Text input border.
            border = {
                widths: 1,
                sides: border_color().map_into(),
            };

            /// When the pointer device is over this text input or it is the return focus.
            when self.is_cap_hovered || self.is_return_focus {
                border = {
                    widths: 1,
                    sides: border_color_hovered().map_into(),
                };
            }

            /// When the text input has keyboard focus.
            when self.is_focused {
                border = {
                    widths: 1,
                    sides: border_color_focused().map_into(),
                };
            }

            /// When the text input is disabled.
            when self.is_disabled {
                saturate = false;
                child_opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}
