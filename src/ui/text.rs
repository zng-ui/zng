use super::{LayoutPoint, LayoutRect, LayoutSize, RenderContext, Ui};
use harfbuzz_rs::*;

pub struct Text {

}

impl Text {
    pub fn new(text: &str) -> Self {
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
    fn render(&self, c: RenderContext) {
        unimplemented!()
    }
}