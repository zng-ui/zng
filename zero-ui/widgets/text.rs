//! Text widgets.

use crate::core::profiler::profile_scope;
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
            text(impl IntoVar<Text>) = "";
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
        text_color as color;
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
            text(impl IntoVar<Text>);
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
            text(impl IntoVar<Text>);
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

    // Loaded from [font query](Fonts::get_or_default) during init.
    font_face: Option<FontFaceRef>,

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
    size: PxSize,
}

impl<T: Var<Text>> TextNode<T> {
    /// New text node from a [`Text`] variable.
    ///
    /// All other text configuration is taken from context variables.
    pub fn new(text: T) -> TextNode<T> {
        TextNode {
            text_var: text,

            text: SegmentedText::default(),
            font_face: None,

            synthesis_used: FontSynthesis::DISABLED,

            line_shaping_args: TextShapingArgs::default(),
            layout_line_spacing: 0.0,
            font: None,
            shaped_text: ShapedText::default(),
            size: PxSize::zero(),
        }
    }
}

#[impl_ui_node(none)]
impl<T: Var<Text>> UiNode for TextNode<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let (family, style, weight, stretch) = TextContext::font_face(ctx.vars);

        // TODO use the full list.
        let font_face = ctx.services.fonts().get_list(family, style, weight, stretch).best().clone();
        self.synthesis_used = *FontSynthesisVar::get(ctx) & font_face.synthesis_for(style, weight);
        self.font_face = Some(font_face);

        let text = self.text_var.get_clone(ctx);
        let text = TextTransformVar::get(ctx).transform(text);
        let text = WhiteSpaceVar::get(ctx).transform(text);
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
        if let Some(text) = self.text_var.get_new(ctx) {
            let (text_transform, white_space) = TextContext::text(ctx);
            let text = text_transform.transform(text.clone());
            let text = white_space.transform(text);
            if self.text.text() != text {
                self.text = SegmentedText::new(text);
                self.shaped_text = ShapedText::default();

                ctx.updates.layout();
            }
        } else if let Some((text_transform, white_space)) = TextContext::text_update(ctx) {
            let text = self.text_var.get_clone(ctx);
            let text = text_transform.transform(text);
            let text = white_space.transform(text);
            if self.text.text() != text {
                self.text = SegmentedText::new(text);
                self.shaped_text = ShapedText::default();

                ctx.updates.layout();
            }
        }

        // update `self.font_face`, affects shaping and layout
        if let Some((font_family, font_style, font_weight, font_stretch)) = TextContext::font_face_update(ctx.vars) {
            let face = ctx
                .services
                .fonts()
                .get_list(font_family, font_style, font_weight, font_stretch)
                .best()
                .clone();

            if !self.font_face.as_ref().map(|f| f.ptr_eq(&face)).unwrap_or_default() {
                self.synthesis_used = *FontSynthesisVar::get(ctx) & face.synthesis_for(font_style, font_weight);
                self.font_face = Some(face);
                self.font = None;
                self.shaped_text = ShapedText::default();

                ctx.updates.layout();
            }
        }

        // update `self.font_instance`, affects shaping and layout
        if TextContext::font_update(ctx).is_some() {
            self.font = None;
            self.shaped_text = ShapedText::default();
            ctx.updates.layout();
        }

        // TODO features, spacing, breaking.

        // update `self.color`
        if TextContext::color_update(ctx).is_some() {
            ctx.updates.render();
        }

        // update `self.font_synthesis`
        if let Some((synthesis_allowed, style, weight)) = TextContext::font_synthesis_update(ctx) {
            if let Some(face) = &self.font_face {
                let synthesis_used = synthesis_allowed & face.synthesis_for(style, weight);
                if synthesis_used != self.synthesis_used {
                    self.synthesis_used = synthesis_used;
                    ctx.updates.render();
                }
            }
        }
    }

    fn measure(&mut self, ctx: &mut LayoutContext, available_size: AvailableSize) -> PxSize {
        let (size, variations) = TextContext::font(ctx);
        let size = size.to_layout(ctx, available_size.width, ctx.metrics.root_font_size);

        if self.font.as_ref().map(|f| f.size() != size).unwrap_or(true) {
            self.font = Some(
                self.font_face
                    .as_ref()
                    .expect("font not inited in measure")
                    .sized(size, variations.finalize()),
            );
        }

        if self.shaped_text.is_empty() {
            // TODO
            let font = self.font.as_ref().unwrap();
            self.shaped_text = font.shape_text(&self.text, &self.line_shaping_args);
            self.size = self.shaped_text.size();
        }

        if available_size.width < self.size.width {
            //TODO wrap here? or estimate the height pos wrap?
        }

        self.size
    }

    fn arrange(&mut self, _ctx: &mut LayoutContext, _final_size: PxSize) {
        // TODO use final size for wrapping?
        // http://www.unicode.org/reports/tr14/tr14-45.html
    }

    fn render(&self, ctx: &mut RenderContext, frame: &mut FrameBuilder) {
        profile_scope!("text::render");
        frame.push_text(
            PxRect::from_size(self.size),
            self.shaped_text.glyphs(),
            self.font.as_ref().expect("font not initied in render"),
            RenderColor::from(*TextColorVar::get(ctx)),
            self.synthesis_used,
        );
    }
}
