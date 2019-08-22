use super::{InitContext, LayoutSize, RenderContext, Ui};
use app_units::Au;
use font_loader::system_fonts;
use webrender::api::*;

pub struct Text {
    glyphs: Vec<GlyphInstance>,
    size: LayoutSize,
    font_instance_key: FontInstanceKey,
    color: ColorF,
}

impl Text {
    pub fn new(c: &InitContext, text: &str, color: ColorF, font_family: &str, font_size: f32) -> Self {
        let font_key = c.api.generate_font_key();
        let property = system_fonts::FontPropertyBuilder::new().family(font_family).build();
        let (font, _) = system_fonts::get(&property).unwrap();

        let mut txn = Transaction::new();
        txn.add_raw_font(font_key, font, 0);

        let font_instance_key = c.api.generate_font_instance_key();
        txn.add_font_instance(
            font_instance_key,
            font_key,
            Au::from_f32_px(font_size),
            None,
            None,
            Vec::new(),
        );

        c.api.send_transaction(c.document_id, txn);

        let indices: Vec<_> = c
            .api
            .get_glyph_indices(font_key, text)
            .into_iter()
            .filter_map(|i| i)
            .collect();
        let dimensions = c.api.get_glyph_dimensions(font_instance_key, indices.clone());

        let mut glyphs = Vec::with_capacity(indices.len());
        let mut offset = 0.;

        assert_eq!(indices.len(), dimensions.len());

        for (index, dim) in indices.into_iter().zip(dimensions) {
            if let Some(dim) = dim {
                glyphs.push(GlyphInstance {
                    index,
                    point: LayoutPoint::new(offset, font_size),
                });

                offset += dim.advance as f32;
            }else{
                offset += font_size/4.;
            }
        }
        let size = LayoutSize::new(offset, font_size*1.3);
        glyphs.shrink_to_fit();

        //https://harfbuzz.github.io/
        //https://crates.io/crates/unicode-bidi
        //https://www.geeksforgeeks.org/word-wrap-problem-dp-19/

        Text {
            glyphs,
            size,
            font_instance_key,
            color,
        }
    }
}

pub fn text(c: &InitContext, text: &str, color: ColorF, font_family: &str, font_size: f32) -> Text {
    Text::new(c, text, color, font_family, font_size)
}

impl Ui for Text {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.size
    }

    fn render(&self, mut c: RenderContext) {
        c.push_text(
            LayoutRect::from_size(self.size),
            &self.glyphs,
            self.font_instance_key,
            self.color,
        )
    }
}
