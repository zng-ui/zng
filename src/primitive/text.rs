use crate::core::*;
use crate::primitive::{SetParentValue, SetParentValueExt};
use std::borrow::Cow;
use webrender::api::*;

pub struct Text<T: Value<Cow<'static, str>>> {
    text: T,
    hit_tag: HitTag,

    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font_instance_key: Option<FontInstanceKey>,
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
            font_instance_key: None,
            color: ColorF::BLACK,
        }
    }

    fn update(&mut self, v: &mut UiValues, u: &mut NextUpdate) {
        if let (Some(font_family), Some(font_size)) = (v.parent(*FONT_FAMILY), v.parent(*FONT_SIZE)) {
            let font = u.font(&font_family, *font_size);

            let indices: Vec<_> = u
                .api
                .get_glyph_indices(font.font_key, &self.text)
                .into_iter()
                .filter_map(|i| i)
                .collect();
            let dimensions = u.api.get_glyph_dimensions(font.instance_key, indices.clone());

            let mut glyphs = Vec::with_capacity(indices.len());
            let mut offset = 0.;

            assert_eq!(indices.len(), dimensions.len());

            for (index, dim) in indices.into_iter().zip(dimensions) {
                if let Some(dim) = dim {
                    glyphs.push(GlyphInstance {
                        index,
                        point: LayoutPoint::new(offset, font.size as f32),
                    });

                    offset += dim.advance as f32;
                } else {
                    offset += font.size as f32 / 4.;
                }
            }
            glyphs.shrink_to_fit();

            self.glyphs = glyphs;
            self.size = LayoutSize::new(offset, font.size as f32 * 1.3);
            self.font_instance_key = Some(font.instance_key);
        } else {
            self.glyphs = vec![];
            self.size = LayoutSize::default();
            self.font_instance_key = None;
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
        if let Some(font) = self.font_instance_key {
            f.push_text(
                LayoutRect::from_size(self.size),
                &self.glyphs,
                font,
                self.color,
                Some(self.hit_tag),
            )
        }
    }
}

pub fn text<T: IntoValue<Cow<'static, str>>>(text: T) -> Text<T::Value> {
    Text::new(text.into_value())
}

pub static FONT_FAMILY: ParentValueKeyRef<Cow<'static, str>> = ParentValueKey::new_lazy();
pub static FONT_SIZE: ParentValueKeyRef<u32> = ParentValueKey::new_lazy();
pub static TEXT_COLOR: ParentValueKeyRef<ColorF> = ParentValueKey::new_lazy();

pub type SetFontFamily<T, R> = SetParentValue<T, Cow<'static, str>, R>;
pub type SetFontSize<T, R> = SetParentValue<T, u32, R>;
pub type SetTextColor<T, R> = SetParentValue<T, ColorF, R>;

pub trait TextVals: Ui + Sized {
    fn font_family<V: IntoValue<Cow<'static, str>>>(self, font: V) -> SetFontFamily<Self, V::Value> {
        self.set_parent_val(*FONT_FAMILY, font)
    }
    fn font_size<V: IntoValue<u32>>(self, size: V) -> SetFontSize<Self, V::Value> {
        self.set_parent_val(*FONT_SIZE, size)
    }
    fn text_color<V: IntoValue<ColorF>>(self, color: V) -> SetTextColor<Self, V::Value> {
        self.set_parent_val(*TEXT_COLOR, color)
    }
}
impl<T: Ui> TextVals for T {}
