use super::{HitTag, Hits, LayoutSize, NextFrame, NextUpdate, UiLeaf};
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
