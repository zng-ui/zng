use super::{LayoutSize, RenderContext, Ui};
//use harfbuzz_rs::*;

pub struct Text {}

impl Text {
    pub fn new(_text: &str) -> Self {
        unimplemented!()
    }
}

pub fn text(text: &str) -> Text {
    Text::new(text)
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
