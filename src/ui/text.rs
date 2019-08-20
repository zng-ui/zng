use super::{LayoutSize, InitContext, RenderContext, Ui};
use font_loader::system_fonts;
use webrender::api::*;
use app_units::Au;


pub struct Text {
    glyphs: Vec<GlyphInstance>,
    size: LayoutSize
}

impl Text {
    pub fn new(c: &InitContext, text: &str) -> Self {
        let font_key = c.api.generate_font_key();
        let property = system_fonts::FontPropertyBuilder::new().family("Arial").build();
        let (font, _) = system_fonts::get(&property).unwrap();

        let mut txn = Transaction::new();
        txn.add_raw_font(font_key, font, 0);

        let font_instance_key = c.api.generate_font_instance_key();
        txn.add_font_instance(font_instance_key, font_key, Au::from_px(32), None, None, Vec::new());

        c.api.send_transaction(c.document_id, txn);

        let indices = c.api.get_glyph_indices(font_key, text).into_iter().filter_map(|i|i).collect();
        let dimensions = c.api.get_glyph_dimensions(font_instance_key, indices);

        let mut size = LayoutSize::default();

        //https://github.com/servo/webrender/blob/master/examples/multiwindow.rs
        //https://docs.rs/webrender_api/0.60.0/webrender_api/struct.RenderApi.html#method.get_glyph_indices
        //https://crates.io/crates/font-loader

        unimplemented!()
    }
}

pub fn text(text: &str) -> Text {
    unimplemented!()//Text::new(text)
}

impl Ui for Text {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        unimplemented!()
    }
    fn arrange(&mut self, _final_size: LayoutSize) {
        unimplemented!()
    }
    fn render(&self, _c: RenderContext) {
        unimplemented!()
    }
}
