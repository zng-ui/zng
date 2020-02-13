use crate::core::context::*;
use crate::core::font::*;
use crate::core::render::FrameBuilder;
use crate::core::types::*;
use crate::core::var::*;
use crate::core::var::{IntoVar, Var};
use crate::core::UiNode;
use crate::impl_ui_node;
use std::borrow::Cow;

struct Text<T: Var<Cow<'static, str>>> {
    text: T,

    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font: Option<FontInstance>,
    color: ColorF,
}

#[impl_ui_node(none)]
impl<T: Var<Cow<'static, str>>> UiNode for Text<T> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        profile_scope!("text::init");

        self.color = *TextColor.get(ctx.vars);
        let font_size = *FontSize.get(ctx.vars);

        let font_family = &FontFamily.get(ctx.vars);
        let font = ctx.window_services.req::<Fonts>().get(font_family, font_size);

        let font_size = font_size as f32;

        let (indices, dimensions) = font.glyph_layout(self.text.get(ctx.vars));
        let mut glyphs = Vec::with_capacity(indices.len());
        let mut offset = 0.;
        assert_eq!(indices.len(), dimensions.len());
        for (index, dimension) in indices.into_iter().zip(dimensions) {
            if let Some(dimension) = dimension {
                glyphs.push(GlyphInstance {
                    index,
                    point: LayoutPoint::new(offset, font_size),
                });
                offset += dimension.advance as f32;
            } else {
                offset += font_size / 4.;
            }
        }
        glyphs.shrink_to_fit();

        self.glyphs = glyphs;
        self.size = LayoutSize::new(offset, font_size * 1.3);
        self.font = Some(font);
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        profile_scope!("text::update");

        if FontFamily.is_new(ctx.vars) || FontSize.is_new(ctx.vars) {
            self.init(ctx);
            ctx.updates.push_layout();
        }

        if let Some(&color) = TextColor.update(ctx.vars) {
            self.color = color;
            ctx.updates.push_render();
        }
    }

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.size
    }

    fn render(&self, frame: &mut FrameBuilder) {
        profile_scope!("text::render");

        frame.push_text(
            LayoutRect::from_size(self.size),
            &self.glyphs,
            self.font.as_ref().unwrap().instance_key(),
            self.color,
            None,
        )
    }
}

/// Simple text run.
///
/// # Context Vars
/// This context variables are used to configure the text:
///
/// * [FontFamily]: Is set by the [font_family](crate::properties::font_family) property.
/// * [FontSize]: Is set by the [font_size](crate::properties::font_family) property.
/// * [TextColor]: Is set by the [text_color](crate::properties::font_family) property.
///
/// # Example
/// ```
/// let hello_txt = container! {
///     font_family: "Arial";
///     font_size: 18;
///     => text("Hello!")
/// }
/// ```
pub fn text(text: impl IntoVar<Cow<'static, str>>) -> impl UiNode {
    Text {
        text: text.into_var(),

        glyphs: vec![],
        size: LayoutSize::default(),
        font: None,
        color: ColorF::BLACK,
    }
}

context_var! {
    /// Font family context var.
    ///
    /// # Text
    /// This context variable is used by the [text](crate::widgets::text::text) widget to
    /// determinate the font family of the text.
    ///
    /// # Default
    /// When not set the value is `Sans-Serif`.
    pub struct FontFamily: Cow<'static, str> = Cow::Borrowed("Sans-Serif");

    /// Font size context var.
    ///
    /// # Text
    /// This context variable is used by the [text](crate::widgets::text::text) widget to
    /// determinate the font size of the text.
    ///
    /// # Default
    /// When not set the value is `14`.
    pub struct FontSize: u32 = 14;

    /// Text color context var.
    ///
    /// # Text
    /// This context variable is used by the [text](crate::widgets::text::text) widget to
    /// determinate the text color.
    ///
    /// # Default
    /// When not set the value is `ColorF::BLACK`.
    pub struct TextColor: ColorF = ColorF::BLACK;
}
