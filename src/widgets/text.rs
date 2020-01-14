use crate::core::*;
use crate::core2::*;
use crate::properties::set_context_var;
use crate::property;
use zero_ui_macros::impl_ui_node_crate;

use std::borrow::Cow;
use webrender::api::*;

pub struct Text<T: Value<Cow<'static, str>>> {
    text: T,
    hit_tag: HitTag,

    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font: Option<FontInstance>,
    color: ColorF,
}

#[impl_ui_crate]
impl<T: Value<Cow<'static, str>>> Text<T> {
    pub fn new(text: T) -> Self {
        //https://harfbuzz.github.io/
        //https://crates.io/crates/unicode-bidi
        //https://www.geeksforgeeks.org/word-wrap-problem-dp-19/
        //https://gankra.github.io/blah/text-hates-you/

        Text {
            text,
            hit_tag: HitTag::new_unique(),

            glyphs: vec![],
            size: LayoutSize::default(),
            font: None,
            color: ColorF::BLACK,
        }
    }

    fn update(&mut self, v: &mut UiValues, u: &mut NextUpdate) {
        if let (Some(font_family), Some(font_size)) = (v.parent(*FONT_FAMILY), v.parent(*FONT_SIZE)) {
            let font = u.font(&font_family, *font_size);
            let font_size = *font_size as f32;

            let (indices, dimensions) = font.glyph_layout(&self.text);
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
        } else {
            self.glyphs = vec![];
            self.size = LayoutSize::default();
            self.font = None;
        }

        self.color = *v.parent(*TEXT_COLOR).unwrap_or(&ColorF::BLACK);
    }

    #[Ui]
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.update(values, update);
    }

    #[Ui]
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.size
    }

    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    #[Ui]
    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if self.text.touched() {
            self.update(values, update);
        }
    }

    #[Ui]
    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.update(values, update);
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        profile_scope!("text_render");
        if let Some(font_instance_key) = self.font.as_ref().map(|f| f.instance_key()) {
            f.push_text(
                LayoutRect::from_size(self.size),
                &self.glyphs,
                font_instance_key,
                self.color,
                Some(self.hit_tag),
            )
        }
    }
}

pub fn text<T: IntoValue<Cow<'static, str>>>(text: T) -> Text<T::Value> {
    Text::new(text.into_value())
}
context_var! {
    pub FontFamily: Cow<'static, str> = "sans-serif".into();
    pub FontSize: u32 = 14;
    pub TextColor: ColorF = ColorF::BLACK;
}

/// Sets the font family for all child Uis.
#[property(context_var)]
pub fn font_family(child: impl UiNode, font: impl IntoVar<Cow<'static, str>>) -> impl UiNode {
    set_context_var::set(child, FontFamily, font)
}

#[property(context_var)]
pub fn font_size(child: impl UiNode, size: impl IntoVar<u32>) -> impl UiNode {
    set_context_var::set(child, FontSize, size)
}

/// Sets the text color for the Ui and its decendents.
#[property(context_var)]
pub fn text_color(child: impl UiNode, color: impl IntoVar<ColorF>) -> impl UiNode {
    set_context_var::set(child, TextColor, color)
}
