//! Text widgets.

use crate::prelude::new_widget::*;
use crate::properties::text_theme::*;

/// A configured [`text`](../fn.text.html).
///
/// # Example
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
/// # `text()`
///
/// If you don't need to configure the text, you can just use the function [`text`](../fn.text.html).
#[widget($crate::widgets::text::text)]
pub mod text {
    use super::*;

    properties! {
        child {
            /// The [`Text`](crate::core::types::Text) value.
            ///
            /// Set to an empty string (`""`).
            text_value as text = "";
        }

        /// The text font. If not set inherits the `font_family` from the parent widget.
        font_family;
        /// The font style. If not set inherits the `font_style` from the parent widget.
        font_style;
        /// The font weight. If not set inherits the `font_weight` from the parent widget.
        font_weight;
        /// The font stretch. If not set inherits the `font_stretch` from the parent widget.
        font_stretch;
        /// The font size. If not set inherits the `font_size` from the parent widget.
        font_size;
        /// The text color. If not set inherits the `text_color` from the parent widget.
        text_color as color ;
        /// Height of each text line. If not set inherits the `line_height` from the parent widget.
        line_height;
    }

    #[inline]
    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        TextNode::new(text.into_var())
    }
}

/// Simple text run.
///
/// # Configure
///
/// Text spans can be configured by setting [`font_family`](crate::properties::text_theme::font_family()),
/// [`font_size`](fn@crate::properties::text_theme::font_size) or [`text_color`](fn@crate::properties::text_theme::text_color)
/// in parent widgets.
///
/// # Example
/// ```
/// # fn main() -> () {
/// use zero_ui::widgets::{container, text::text};
/// use zero_ui::properties::text_theme::{font_family, font_size};
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
/// There is a specific widget for creating configured text runs: [`text!`](text/index.html).
pub fn text(text: impl IntoVar<Text> + 'static) -> impl Widget {
    // TODO remove 'static when rust issue #42940 is fixed.
    text! {
        text;
    }
}

#[widget($crate::widgets::text::strong)]
mod strong {
    use super::*;

    properties! {
        child {
            text_value as text;
        }
    }

    #[inline]
    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        let text = TextNode::new(text.into_var());
        font_weight(text, FontWeight::BOLD)
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

#[widget($crate::widgets::text::em)]
mod em {
    use super::*;

    properties! {
        child {
            text_value as text;
        }
    }

    #[inline]
    fn new_child(text: impl IntoVar<Text>) -> impl UiNode {
        let text = TextNode::new(text.into_var());
        font_style(text, FontStyle::Italic)
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

/// An UI node that renders a text using the [contextual text theme](TextContext).
pub struct TextNode<T: Var<Text>> {
    text_var: T,

    /* init, update data */
    // Transformed and white space corrected, or empty before init.
    text: SegmentedText,
    // Copy for render, or black before init.
    color: RenderColor,
    // Loaded from [font query](Fonts::get_or_default) during init.
    font_face: Option<FontFaceRef>,
    // Copy for layout, or zero before init.
    font_size: Length,
    #[allow(unused)] // TODO
    line_spacing: Length,

    synthesis_used: FontSynthesis,

    /* measure, arrange data */
    //
    line_shaping_args: TextShapingArgs,

    #[allow(unused)] // TODO
    layout_line_spacing: f32,
    // Font instance using the actual font_size.
    font: Option<FontRef>,
    // Shaped and wrapped text.
    shaped_text: ShapedText,
    // Box size of the text block.
    size: LayoutSize,
}

impl<T: Var<Text>> TextNode<T> {
    pub fn new(text: T) -> TextNode<T> {
        TextNode {
            text_var: text,

            text: SegmentedText::default(),
            color: colors::BLACK.into(),
            font_face: None,

            font_size: 0.into(),
            line_spacing: 0.into(),

            synthesis_used: FontSynthesis::DISABLED,

            line_shaping_args: TextShapingArgs::default(),
            layout_line_spacing: 0.0,
            font: None,
            shaped_text: ShapedText::default(),
            size: LayoutSize::zero(),
        }
    }
}

#[impl_ui_node(none)]
impl<T: Var<Text>> UiNode for TextNode<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let t_ctx = TextContext::get(ctx.vars);

        // TODO use the full list.
        let font_face = ctx
            .services
            .req::<Fonts>()
            .get_list(t_ctx.font_family, t_ctx.font_style, t_ctx.font_weight, t_ctx.font_stretch)
            .best()
            .clone();
        self.synthesis_used = t_ctx.font_synthesis & font_face.synthesis_for(t_ctx.font_style, t_ctx.font_weight);
        self.font_face = Some(font_face);

        self.font_size = t_ctx.font_size;

        self.color = t_ctx.text_color.into();

        let text = self.text_var.get(ctx.vars).clone();
        let text = t_ctx.text_transform.transform(text);
        let text = t_ctx.white_space.transform(text);
        self.text = SegmentedText::new(text)
    }

    fn deinit(&mut self, _: &mut WidgetContext) {
        self.font = None;
        self.font_face = None;
        self.shaped_text = ShapedText::default();
        self.text = SegmentedText::default();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        // update `self.text`, affects shaping and layout
        if let Some(text) = self.text_var.get_new(ctx.vars) {
            let (text_transform, white_space) = TextContext::text(ctx.vars);
            let text = text_transform.transform(text.clone());
            let text = white_space.transform(text);
            if self.text.text() != text {
                self.text = SegmentedText::new(text);
                self.shaped_text = ShapedText::default();

                ctx.updates.layout();
            }
        } else if let Some((text_transform, white_space)) = TextContext::text_update(ctx.vars) {
            let text = self.text_var.get(ctx.vars).clone();
            let text = text_transform.transform(text);
            let text = white_space.transform(text);
            if self.text.text() != text {
                self.text = SegmentedText::new(text);
                self.shaped_text = ShapedText::default();

                ctx.updates.layout();
            }
        }

        // update `self.font_face`, affects shaping and layout
        if let Some((font_family, font_style, font_weight, font_stretch)) = TextContext::font_fate_update(ctx.vars) {
            let face = ctx
                .services
                .req::<Fonts>()
                .get_list(font_family, font_style, font_weight, font_stretch)
                .best()
                .clone();

            if !self.font_face.as_ref().map(|f| f.ptr_eq(&face)).unwrap_or_default() {
                self.synthesis_used = *FontSynthesisVar::var().get(ctx.vars) & face.synthesis_for(font_style, font_weight);
                self.font_face = Some(face);
                self.font = None;
                self.shaped_text = ShapedText::default();

                ctx.updates.layout();
            }
        }

        // update `self.font_instance`, affects shaping and layout
        if let Some(font_size) = TextContext::font_update(ctx.vars) {
            if font_size != self.font_size {
                self.font_size = font_size;

                self.font = None;
                self.shaped_text = ShapedText::default();

                ctx.updates.layout();
            }
        }

        // TODO features, spacing, breaking.

        // update `self.color`
        if let Some(color) = TextContext::color_update(ctx.vars) {
            let color = RenderColor::from(color);
            if self.color != color {
                self.color = color;

                ctx.updates.render();
            }
        }

        // update `self.font_synthesis`
        if let Some((synthesis_allowed, style, weight)) = TextContext::font_synthesis_update(ctx.vars) {
            if let Some(face) = &self.font_face {
                let synthesis_used = synthesis_allowed & face.synthesis_for(style, weight);
                if synthesis_used != self.synthesis_used {
                    self.synthesis_used = synthesis_used;
                    ctx.updates.render();
                }
            }
        }
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        if self.font.is_none() {
            let size = self.font_size.to_layout(LayoutLength::new(available_size.width), ctx);
            self.font = Some(self.font_face.as_ref().expect("font not inited in measure").sized(size));
        };

        if self.shaped_text.is_empty() {
            // TODO
            let font = self.font.as_ref().unwrap();
            self.shaped_text = font.shape_text(&self.text, &self.line_shaping_args);
            self.size = self.shaped_text.size().snap_to(ctx.pixel_grid());
        }

        if !is_layout_any_size(available_size.width) && available_size.width < self.size.width {
            //TODO wrap here? or estimate the height pos wrap?
        }

        self.size
    }

    fn arrange(&mut self, _final_size: LayoutSize, _ctx: &mut LayoutContext) {
        // TODO use final size for wrapping?
        // http://www.unicode.org/reports/tr14/tr14-45.html
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.push_text(
            LayoutRect::from_size(self.size),
            self.shaped_text.glyphs(),
            self.font.as_ref().expect("font not initied in render"),
            self.color,
            self.synthesis_used,
        );
    }
}
