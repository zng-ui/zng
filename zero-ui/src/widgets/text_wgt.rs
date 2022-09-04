use crate::prelude::new_widget::*;

pub mod nodes;
pub mod properties;

/// Common text properties.
#[widget_mixin($crate::widgets::text_mixin)]
pub mod text_mixin {
    use super::*;

    properties! {
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
}

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

    inherit!(super::text_mixin);

    properties! {
        /// The [`Text`](crate::core::types::Text) value.
        ///
        /// Set to an empty string (`""`) by default.
        text(impl IntoVar<Text>) = "";

        /// Spacing in between the text and background edges or border.
        ///
        /// Set to `0` by default.
        padding = 0;
    }

    fn new_child() -> impl UiNode {
        let child = nodes::render_text();
        let child = nodes::render_caret(child);
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

    use crate::widgets::themable;

    inherit!(themable);
    inherit!(super::text_mixin);

    properties! {
        /// The [`Text`](crate::core::types::Text) value.
        ///
        /// Set to an empty string (`""`) by default.
        text(impl IntoVar<Text>) = "";

        /// Enabled by default.
        editable = true;

        /// Enabled by default.
        ///
        /// Blocks pointer interaction with other widgets while the text input is pressed.
        capture_mouse = true;

        /// Enables keyboard focusing in the widget.
        focusable = true;

        /// Text input dark and light themes.
        ///
        /// Set to [`theme::pair`] of [`vis::DARK_THEME_VAR`], [`vis::LIGHT_THEME_VAR`] by default.
        theme = theme::pair(vis::DARK_THEME_VAR, vis::LIGHT_THEME_VAR);
    }

    fn new_child() -> impl UiNode {
        let child = nodes::render_text();
        let child = nodes::render_caret(child);
        let child = nodes::render_overlines(child);
        let child = nodes::render_strikethroughs(child);
        nodes::render_underlines(child)
    }

    fn new_fill_dyn(child: impl UiNode, part: DynWidgetPart) -> impl UiNode {
        let child = nodes::layout_text(child, vis::PADDING_VAR);
        themable::new_fill_dyn(child, part)
    }

    fn new_event_dyn(child: impl UiNode, part: DynWidgetPart, text: impl IntoVar<Text>) -> impl UiNode {
        let child = nodes::resolve_text(child, text);
        themable::new_event_dyn(child, part)
    }

    #[doc(inline)]
    pub use super::text_input_vis as vis;
}
/// Text input themes.
pub mod text_input_vis {
    use super::*;

    use crate::widgets::text::properties::TEXT_COLOR_VAR;

    /// Text input base theme.
    #[widget($crate::widgets::text_input::vis::base_theme)]
    pub mod base_theme {
        use super::*;

        inherit!(theme);

        properties! {
            /// Button padding.
            ///
            /// Is `(7, 15)` by default.
            padding = (7, 15);
        }
    }

    /// Default text input dark theme.
    #[widget($crate::widgets::text_input::vis::dark_theme)]
    pub mod dark_theme {
        use super::*;

        inherit!(base_theme);

        properties! {
            /// Text input base color, all background and border colors are derived from this color.
            dark_color as base_color;

            /// Text input background color.
            ///
            /// Is the base color by default.
            background_color = rgb(0.12, 0.12, 0.12);

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 30%.
            border = {
                widths: 1,
                sides: DARK_COLOR_VAR.map_into(),
            };

            /// When the pointer device is over this button.
            when self.is_cap_hovered {
                border = {
                    widths: 1,
                    sides: dark_color_hovered().map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_focused /*|| self.is_return_focused */  {
                border = {
                    widths: 1,
                    sides: dark_color_focused().map_into(),
                };
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = dark_color_disabled();
                border = {
                    widths: 1,
                    sides: dark_color_hovered().map_into(),
                };
                text_color = TEXT_COLOR_VAR.map(|&c| colors::BLACK.with_alpha(0.5).mix_normal(c));
                cursor = CursorIcon::NotAllowed;
            }
        }
    }

    /// Default text input light theme.
    #[widget($crate::widgets::text_input::vis::light_theme)]
    pub mod light_theme {
        use super::*;

        inherit!(base_theme);

        properties! {
            /// Text input base color, all background and border colors are derived from this color.
            light_color as base_color;

            /// Text input background color.
            ///
            /// Is the base color by default.
            background_color = rgb(0.88, 0.88, 0.88);

            /// Button border.
            ///
            /// Is widths `1` and sides the base color lighten by 30%.
            border = {
                widths: 1,
                sides: LIGHT_COLOR_VAR.map_into(),
            };

            /// When the pointer device is over this button or it is the return focus of a scope.
            when self.is_cap_hovered /*|| self.is_return_focused */ {
                border = {
                    widths: 1,
                    sides: light_color_hovered().map_into(),
                };
            }

            /// When the button is pressed in a way that press release will cause a button click.
            when self.is_focused {
                border = {
                    widths: 1,
                    sides: light_color_focused().map_into(),
                };
            }

            /// When the button is disabled.
            when self.is_disabled {
                background_color = light_color_disabled();
                border = {
                    widths: 1,
                    sides: light_color_hovered().map_into(),
                };
                text_color = TEXT_COLOR_VAR.map(|&c| colors::WHITE.with_alpha(0.5).mix_normal(c));
                cursor = CursorIcon::NotAllowed;
            }
        }
    }

    context_var! {
        /// Text input dark theme.
        ///
        /// Use the [`text_input::vis::dark`] property to set.
        ///
        /// [`text_input::vis::dark`]: fn@dark
        pub static DARK_THEME_VAR: ThemeGenerator = ThemeGenerator::new(|_, _| dark_theme!());

        /// Text input light theme.
        ///
        /// Use the [`text_input::vis::light`] property to set.
        ///
        /// [`text_input::vis::light`]: fn@light
        pub static LIGHT_THEME_VAR: ThemeGenerator = ThemeGenerator::new(|_, _| light_theme!());

        /// Idle border color in the dark theme.
        ///
        /// All other border states are derived by adjusting the brightness of this color.
        pub static DARK_COLOR_VAR: Rgba = rgb(0.3, 0.3, 0.3);

        /// Idle border color in the light theme.
        ///
        /// All other border states are derived by adjusting the brightness of this color.
        pub static LIGHT_COLOR_VAR: Rgba = rgb(0.7, 0.7, 0.7);

        /// Text input padding.
        ///
        /// Text padding is computed directly on the text layout,
        pub static PADDING_VAR: SideOffsets = 0.into();
    }

    /// Sets the [`DARK_THEME_VAR`] that affects all text inputs inside the widget.
    #[property(context, default(DARK_THEME_VAR))]
    pub fn dark(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, DARK_THEME_VAR, theme)
    }

    /// Sets the [`LIGHT_THEME_VAR`] that affects all text inputs inside the widget.
    #[property(context, default(LIGHT_THEME_VAR))]
    pub fn light(child: impl UiNode, theme: impl IntoVar<ThemeGenerator>) -> impl UiNode {
        with_context_var(child, LIGHT_THEME_VAR, theme)
    }

    /// Sets the [`DARK_COLOR_VAR`] that is used to compute all background and border colors in the text input's dark theme.
    #[property(context, default(DARK_COLOR_VAR))]
    pub fn dark_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, DARK_COLOR_VAR, color)
    }

    /// Sets the [`LIGH_COLOR_VAR`] that is used to compute all background and border colors in the text input's light theme.
    #[property(context, default(LIGHT_COLOR_VAR))]
    pub fn light_color(child: impl UiNode, color: impl IntoVar<Rgba>) -> impl UiNode {
        with_context_var(child, LIGHT_COLOR_VAR, color)
    }

    /// Sets the [`PADDING_VAR`] that is used in the text-input layout.
    #[property(context, default(PADDING_VAR))]
    pub fn padding(child: impl UiNode, padding: impl IntoVar<SideOffsets>) -> impl UiNode {
        with_context_var(child, PADDING_VAR, padding)
    }

    /// Dark border hovered.
    pub fn dark_color_hovered() -> impl Var<Rgba> {
        DARK_COLOR_VAR.map(|&c| colors::WHITE.with_alpha(0.2).mix_normal(c))
    }

    /// Dark border focused.
    pub fn dark_color_focused() -> impl Var<Rgba> {
        DARK_COLOR_VAR.map(|&c| colors::WHITE.with_alpha(0.35).mix_normal(c))
    }

    /// Dark background disabled.
    pub fn dark_color_disabled() -> impl Var<Rgba> {
        DARK_COLOR_VAR.map(|&c| c.desaturate(100.pct()))
    }

    /// Dark background hovered.
    pub fn light_color_hovered() -> impl Var<Rgba> {
        LIGHT_COLOR_VAR.map(|&c| colors::BLACK.with_alpha(0.2).mix_normal(c))
    }

    /// Dark border focused.
    pub fn light_color_focused() -> impl Var<Rgba> {
        LIGHT_COLOR_VAR.map(|&c| colors::BLACK.with_alpha(0.35).mix_normal(c))
    }

    /// Dark background disabled.
    pub fn light_color_disabled() -> impl Var<Rgba> {
        LIGHT_COLOR_VAR.map(|&c| c.desaturate(100.pct()))
    }
}
