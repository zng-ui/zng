use crate::core::*;

#[derive(new)]
pub struct Cursor<T: Ui> {
    child: T,
    cursor: CursorIcon,
}

#[impl_ui_crate(child)]
impl<T: Ui + 'static> Ui for Cursor<T> {
    fn render(&self, f: &mut NextFrame) {
        f.push_cursor(self.cursor, &self.child)
    }
}

pub fn cursor<T: Ui>(child: T, cursor: CursorIcon) -> Cursor<T> {
    Cursor::new(child, cursor)
}

pub trait CursorExt: Ui + Sized {
    fn cursor(self, cursor: CursorIcon) -> Cursor<Self> {
        Cursor::new(self, cursor)
    }
}
impl<T: Ui> CursorExt for T {}
