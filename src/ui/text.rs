use super::{
    HitTag, Hits, IntoValue, LayoutSize, NextFrame, NextUpdate, ParentValue, ParentValueKey, ParentValueKeyRef,
    SetParentValue, Ui, UiValues, impl_ui_crate
};
use webrender::api::*;

pub struct Text {
    text: String,
    hit_tag: HitTag,

    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font_instance_key: Option<FontInstanceKey>,
    color: ColorF,
}

#[impl_ui_crate]
impl Text {
    pub fn new(text: &str) -> Self {
        //https://harfbuzz.github.io/
        //https://crates.io/crates/unicode-bidi
        //https://www.geeksforgeeks.org/word-wrap-problem-dp-19/

        Text {
            text: text.to_owned(),
            hit_tag: HitTag::new(),

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
    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.update(values, update);
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
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

pub fn text(text: &str) -> Text {
    Text::new(text)
}


pub static FONT_FAMILY: ParentValueKeyRef<String> = ParentValueKey::new_lazy();
pub static FONT_SIZE: ParentValueKeyRef<u32> = ParentValueKey::new_lazy();
pub static TEXT_COLOR: ParentValueKeyRef<ColorF> = ParentValueKey::new_lazy();

pub type SetFontFamily<T, R> = SetParentValue<T, String, R>;
pub type SetFontSize<T, R> = SetParentValue<T, u32, R>;
pub type SetTextColor<T, R> = SetParentValue<T, ColorF, R>;

pub trait TextVals: Ui + Sized {
    fn font_family<V: IntoValue<String>>(self, font: V) -> SetFontFamily<Self, V::Value> {
        self.set_ctx_val(*FONT_FAMILY, font)
    }
    fn font_size<V: IntoValue<u32>>(self, size: V) -> SetFontSize<Self, V::Value> {
        self.set_ctx_val(*FONT_SIZE, size)
    }
    fn text_color<V: IntoValue<ColorF>>(self, color: V) -> SetTextColor<Self, V::Value> {
        self.set_ctx_val(*TEXT_COLOR, color)
    }
}
impl<T: Ui> TextVals for T {}
