use crate::core2::*;
use crate::properties::set_context_var;
use crate::{impl_ui_node, property};
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
    fn init(&mut self, ctx: &mut AppContext) {
        self.color = *TextColor.get(ctx);
        let font_size = *FontSize.get(ctx);

        let font_family = &FontFamily.get(ctx);
        let font = ctx.service::<Fonts>().get(font_family, font_size);

        let font_size = font_size as f32;

        let (indices, dimensions) = font.glyph_layout(self.text.get(ctx));
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

    fn update(&mut self, ctx: &mut AppContext) {
        if FontFamily.is_new(ctx) || FontSize.is_new(ctx) {
            self.init(ctx);
            ctx.push_layout();
        }

        if let Some(&color) = TextColor.update(ctx) {
            self.color = color;
            ctx.push_frame();
        }
    }

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.size
    }

    fn render(&self, frame: &mut FrameBuilder) {
        profile_scope!("text_render");

        frame.push_text(
            &LayoutRect::from_size(self.size),
            &self.glyphs,
            self.font.as_ref().unwrap().instance_key(),
            self.color,
            None,
        )
    }
}

/// Simple text run.
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
    /// Font family name.
    ///
    /// # Default
    /// When not set the value is `Sans-Serif`.
    pub struct FontFamily: Cow<'static, str> = Cow::Borrowed("Sans-Serif");

    /// Font size.
    ///
    /// # Default
    /// When not set the value is `14`.
    pub struct FontSize: u32 = 14;

    /// Text color.
    ///
    /// # Default
    /// When not set the value is `ColorF::BLACK`.
    pub struct TextColor: ColorF = ColorF::BLACK;
}

/// Sets the [font family](FontFamily).
#[property(context_var)]
pub fn font_family(child: impl UiNode, font: impl IntoVar<Cow<'static, str>>) -> impl UiNode {
    set_context_var::set(child, FontFamily, font)
}

/// Sets the [font size](FontSize).
#[property(context_var)]
pub fn font_size(child: impl UiNode, size: impl IntoVar<u32>) -> impl UiNode {
    set_context_var::set(child, FontSize, size)
}

/// Sets the [text color](TextColor).
#[property(context_var)]
pub fn text_color(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    set_context_var::set(child, TextColor, color)
}
