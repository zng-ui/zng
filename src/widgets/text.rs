//! Text widgets.

use std::mem;

use crate::core::context::*;
use crate::core::impl_ui_node;
use crate::core::render::FrameBuilder;
use crate::core::text::*;
use crate::core::types::*;
use crate::core::units::*;
use crate::core::var::{IntoVar, Var};
use crate::core::{
    color::{web_colors, RenderColor},
    is_layout_any_size,
};
use crate::core::{UiNode, Widget};
use crate::properties::{capture_only::text_value, text_theme::*};
use zero_ui_macros::widget;

widget! {
    /// A configured [`text`](../fn.text.html).
    ///
    /// # Example
    ///
    /// ```
    /// use zero_ui::widgets::text;
    ///
    /// let hello_txt = text! {
    ///     font_family: "Arial";
    ///     font_size: 18;
    ///     text: "Hello!";
    /// };
    /// ```
    /// # `text()`
    ///
    /// If you don't need to configure the text, you can just use the function [`text`](../fn.text.html).
    pub text;

    default_child {
        /// The [`Text`](crate::core::types::Text) value.
        ///
        /// Set to an empty string (`""`).
        text -> text_value: "";
    }

    default {
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
        color -> text_color;
        /// Height of each text line. If not set inherits the `line_height` from the parent widget.
        line_height;
    }

    /// Creates a [`text`](../fn.text.html).
    #[inline]
    fn new_child(text) -> impl UiNode {
        TextNode::new(text.unwrap().into_var())
    }
}

/// Simple text run.
///
/// # Configure
///
/// Text spans can be configured by setting [`font_family`](crate::properties::text_theme::font_family),
/// [`font_size`](crate::properties::text_theme::font_size) or [`text_color`](crate::properties::text_theme::text_color)
/// in parent widgets.
///
/// # Example
/// ```
/// # fn main() -> () {
/// use zero_ui::widgets::{container, text::text};
/// use zero_ui::properties::text_theme::{font_family, font_size};
///
/// let hello_txt = container! {
///     font_family: "Arial";
///     font_size: 18;
///     content: text("Hello!");
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

widget! {
    strong;

    default_child {
        text -> text_value;
    }

    #[inline]
    fn new_child(text) -> impl UiNode {
        let text = TextNode::new(text.unwrap().into_var());
        font_weight::set(text, FontWeight::BOLD)
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

widget! {
    em;

    default_child {
        text -> text_value;
    }

    #[inline]
    fn new_child(text) -> impl UiNode {
        let text = TextNode::new(text.unwrap().into_var());
        font_style::set(text, FontStyle::Italic)
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
    text: Text,
    // Copy for render, or black before init.
    color: RenderColor,
    // Loaded from [font query](Fonts::get_or_default) during init.
    font: Option<FontRef>,
    // Copy for layout, or zero before init.
    font_size: Length,
    font_synthesis: FontSynthesis,
    line_spacing: Length,

    /* measure, arrange data */
    //
    line_shaping_args: LineShapingArgs,
    layout_line_spacing: f32,
    // Font instance using the actual font_size.
    font_instance: Option<FontInstanceRef>,
    // Shaped and wrapped text.
    shaped_text: Vec<ShapedLine>,
    // All the lines as a single block of glyphs.
    arranged_text: Vec<GlyphInstance>,
    // Box size of the text block.
    size: LayoutSize,
}

impl<T: Var<Text>> TextNode<T> {
    pub fn new(text: T) -> TextNode<T> {
        TextNode {
            text_var: text.into_var(),

            text: "".into(),
            color: web_colors::BLACK.into(),
            font: None,
            font_size: 0.into(),
            font_synthesis: FontSynthesis::DISABLED,
            line_spacing: 0.into(),

            line_shaping_args: LineShapingArgs::default(),
            layout_line_spacing: 0.0,
            font_instance: None,
            shaped_text: vec![],
            arranged_text: vec![],
            size: LayoutSize::zero(),
        }
    }
}

#[impl_ui_node(none)]
impl<T: Var<Text>> UiNode for TextNode<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let t_ctx = TextContext::get(ctx.vars);

        self.font = Some(ctx.window_services.req::<Fonts>().get_or_default(
            t_ctx.font_family,
            t_ctx.font_style,
            t_ctx.font_weight,
            t_ctx.font_stretch,
        ));

        self.font_size = t_ctx.font_size;

        self.color = t_ctx.text_color.into();

        let text = self.text_var.get(ctx.vars).clone();
        let text = t_ctx.text_transform.transform(text);
        self.text = t_ctx.white_space.transform(text);
    }

    fn deinit(&mut self, _: &mut WidgetContext) {
        self.font_instance = None;
        self.font = None;
        self.shaped_text.clear();
        self.text = "".into();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        // update `self.text`, affects shaping and layout
        if let Some(text) = self.text_var.update(ctx.vars) {
            let (text_transform, white_space) = TextContext::text(ctx.vars);
            let text = text_transform.transform(text.clone());
            let text = white_space.transform(text);
            if self.text != text {
                self.text = text;
                self.shaped_text.clear();

                ctx.updates.push_layout();
            }
        } else if let Some((text_transform, white_space)) = TextContext::text_update(ctx.vars) {
            let text = self.text_var.get(ctx.vars).clone();
            let text = text_transform.transform(text);
            let text = white_space.transform(text);
            if self.text != text {
                self.text = text;
                self.shaped_text.clear();

                ctx.updates.push_layout();
            }
        }

        // update `self.font`, affects shaping and layout
        if let Some((font_family, font_style, font_weight, font_stretch)) = TextContext::font_update(ctx.vars) {
            let font = Some(
                ctx.window_services
                    .req::<Fonts>()
                    .get_or_default(font_family, font_style, font_weight, font_stretch),
            );

            if self.font != font {
                self.font = font;
                self.font_instance = None;
                self.shaped_text.clear();

                ctx.updates.push_layout();
            }
        }

        // update `self.font_instance`, affects shaping and layout
        if let Some((font_size, font_synthesis)) = TextContext::font_instance_update(ctx.vars) {
            if font_size != self.font_size || font_synthesis != self.font_synthesis {
                self.font_size = font_size;
                self.font_synthesis = font_synthesis;

                self.font_instance = None;
                self.shaped_text.clear();

                ctx.updates.push_layout();
            }
        }

        // TODO features, spacing, breaking.

        // update `self.color` and `self.glyph_options`, affects render
        if let Some(color) = TextContext::render_update(ctx.vars) {
            let color = RenderColor::from(color);
            if self.color != color {
                self.color = color;

                ctx.updates.push_render();
            }
        }
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        if self.font_instance.is_none() {
            let size = self.font_size.to_layout(LayoutLength::new(available_size.width), ctx);
            let size = font_size_from_layout_length(size);
            self.font_instance = Some(
                self.font
                    .as_ref()
                    .expect("font not inited in measure")
                    .instance(size, self.font_synthesis),
            );
        };

        if self.shaped_text.is_empty() {
            // TODO
            let font = self.font_instance.as_ref().unwrap();
            let mut size = LayoutSize::zero();

            self.shaped_text = self
                .text
                .lines()
                .map(|l| {
                    let l = font.shape_line(l, &self.line_shaping_args);
                    size.width = l.bounds.width.max(size.width);
                    size.height += l.bounds.height; //TODO + line spacing.
                    l
                })
                .collect();

            self.size = size.snap_to(ctx.pixel_grid());
            self.arranged_text.clear();
        }

        if !is_layout_any_size(available_size.width) && available_size.width < self.size.width {
            //TODO wrap here? or estimate the height pos wrap?
        }

        self.size
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        // TODO use final size for wrapping?
        // http://www.unicode.org/reports/tr14/tr14-45.html
        if self.arranged_text.is_empty() && !self.text.is_empty() {
            debug_assert!(!self.shaped_text.is_empty(), "expected at least one empty line in arrange");
            self.arranged_text.extend(mem::take(&mut self.shaped_text[0].glyphs));

            let mut y_offset = self.shaped_text[0].bounds.height + self.layout_line_spacing;
            for line in &mut self.shaped_text[1..] {
                let mut glyphs = mem::take(&mut line.glyphs);
                for g in &mut glyphs {
                    g.point.y += y_offset;
                }
                self.arranged_text.extend(glyphs);
                y_offset += line.bounds.height + self.layout_line_spacing;
            }
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let f_key = self
            .font_instance
            .as_ref()
            .expect("font instanced not inited in render")
            .instance_key();
        //TODO synthetic oblique.
        frame.push_text(LayoutRect::from_size(self.size), &self.arranged_text, f_key, self.color);
    }
}
