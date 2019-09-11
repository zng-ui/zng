use super::{HitTag, Hits, LayoutSize, NextFrame, NextUpdate, Static, Ui, UiContainer, UiLeaf};
use crate::ui::ContextVarKey;
use crate::ui::ReadValue;
use webrender::api::*;

pub struct Text {
    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font_instance_key: FontInstanceKey,
    color: ColorF,
    hit_tag: HitTag,
}

impl Text {
    pub fn new(c: &mut NextUpdate, text: &str, color: ColorF, font_family: &str, font_size: u32) -> Self {
        let font = c.font(font_family, font_size);

        let indices: Vec<_> = c
            .api
            .get_glyph_indices(font.font_key, text)
            .into_iter()
            .filter_map(|i| i)
            .collect();
        let dimensions = c.api.get_glyph_dimensions(font.instance_key, indices.clone());

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
        let size = LayoutSize::new(offset, font.size as f32 * 1.3);
        glyphs.shrink_to_fit();

        //https://harfbuzz.github.io/
        //https://crates.io/crates/unicode-bidi
        //https://www.geeksforgeeks.org/word-wrap-problem-dp-19/

        Text {
            glyphs,
            size,
            font_instance_key: font.instance_key,
            color,
            hit_tag: HitTag::new(),
        }
    }
}

pub fn text(c: &mut NextUpdate, text: &str, color: ColorF, font_family: &str, font_size: u32) -> Text {
    Text::new(c, text, color, font_family, font_size)
}

impl UiLeaf for Text {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.size
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    fn render(&self, f: &mut NextFrame) {
        f.push_text(
            LayoutRect::from_size(self.size),
            &self.glyphs,
            self.font_instance_key,
            self.color,
            Some(self.hit_tag),
        )
    }
}
delegate_ui!(UiLeaf, Text);

lazy_static! {
    static ref FONT_FAMILY: ContextVarKey<String> = ContextVarKey::new();
    static ref FONT_SIZE: ContextVarKey<u32> = ContextVarKey::new();
}

pub struct FontFamily<T: Ui, F: ReadValue<String>> {
    child: T,
    font: F,
}
impl<T: Ui, F: ReadValue<String>> FontFamily<T, F> {
    fn new(child: T, font: F) -> Self {
        FontFamily { child, font }
    }
}
impl<T: Ui, F: ReadValue<String>> UiContainer for FontFamily<T, F> {
    delegate_child!(child, T);

    fn value_changed(&mut self, update: &mut NextUpdate) {
        if self.font.changed() {
            let font_value = self.font.value().clone();
            update.propagate_context_var(*FONT_FAMILY, font_value, self.child_mut());
        }
    }
}
impl<T: Ui, F: ReadValue<String>> Ui for FontFamily<T, F> {
    delegate_ui_methods!(UiContainer);
}

pub struct FontSize<T: Ui, S: ReadValue<u32>> {
    child: T,
    size: S,
}
impl<T: Ui, S: ReadValue<u32>> FontSize<T, S> {
    fn new(child: T, size: S) -> Self {
        FontSize { child, size }
    }
}
impl<T: Ui, S: ReadValue<u32>> UiContainer for FontSize<T, S> {
    delegate_child!(child, T);

    fn value_changed(&mut self, update: &mut NextUpdate) {
        if self.size.changed() {
            let font_value = self.size.value().clone();
            update.propagate_context_var(*FONT_SIZE, font_value, self.child_mut());
        }
    }
}
impl<T: Ui, S: ReadValue<u32>> Ui for FontSize<T, S> {
    delegate_ui_methods!(UiContainer);
}

pub trait Font: Ui + Sized {
    fn font_family(self, font: impl ToString) -> FontFamily<Self, Static<String>> {
        FontFamily::new(self, Static(font.to_string()))
    }
    fn font_size(self, size: u32) -> FontSize<Self, Static<u32>> {
        FontSize::new(self, Static(size))
    }

    fn font_family_dyn<F: ReadValue<String>>(self, font: F) -> FontFamily<Self, F> {
        FontFamily::new(self, font)
    }
    fn font_size_dyn<S: ReadValue<u32>>(self, size: S) -> FontSize<Self, S> {
        FontSize::new(self, size)
    }
}
impl<T: Ui> Font for T {}
