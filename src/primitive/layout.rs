use crate::core::*;
use webrender::euclid;

/// Constrain a child to a size.
/// # Constructors
/// Can be initialized using [`size(child, size)` function](size) and [`child.size(size)`](ExactSize::size).
#[derive(Clone, new)]
pub struct UiSize<T: Ui, S: Value<LayoutSize>> {
    child: T,
    size: S,
}

#[impl_ui_crate(child)]
impl<T: Ui, S: Value<LayoutSize>> UiSize<T, S> {
    #[Ui]
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        let size = *self.size;
        self.child.measure(size);
        size
    }
}

#[derive(Clone, new)]
pub struct UiWidth<T: Ui> {
    child: T,
    width: f32,
}

#[impl_ui_crate(child)]
impl<T: Ui> UiWidth<T> {
    #[Ui]
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.width = self.width;
        let mut child_size = self.child.measure(available_size);
        child_size.width = self.width;
        child_size
    }
}

#[derive(Clone, new)]
pub struct UiHeight<T: Ui> {
    child: T,
    height: f32,
}
#[impl_ui_crate(child)]
impl<T: Ui> UiHeight<T> {
    #[Ui]
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.height = self.height;
        let mut child_size = self.child.measure(available_size);
        child_size.height = self.height;
        child_size
    }
}

pub fn width(child: impl Ui, width: f32) -> impl Ui {
    UiWidth::new(child, width)
}

pub fn height(child: impl Ui, height: f32) -> impl Ui {
    UiHeight::new(child, height)
}

pub fn size(child: impl Ui, size: impl IntoValue<LayoutSize>) -> impl Ui {
    UiSize::new(child, size.into_value())
}

#[derive(Debug, Clone, Copy)]
pub struct Alignment(pub f32, pub f32);

#[derive(Clone, new)]
pub struct Align<T: Ui, A: Value<Alignment>> {
    child: T,
    alignment: A,
    #[new(default)]
    child_rect: LayoutRect,
}

#[impl_ui_crate(child)]
impl<T: Ui, A: Value<Alignment>> Align<T, A> {
    #[Ui]
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        self.child_rect.size = self.child.measure(available_size);

        if available_size.width.is_infinite() {
            available_size.width = self.child_rect.size.width;
        }

        if available_size.height.is_infinite() {
            available_size.height = self.child_rect.size.height;
        }

        available_size
    }

    #[Ui]
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child_rect.size = self.child_rect.size.min(final_size);
        self.child.arrange(self.child_rect.size);

        let alignment = *self.alignment;

        self.child_rect.origin = LayoutPoint::new(
            (final_size.width - self.child_rect.size.width) * alignment.0,
            (final_size.height - self.child_rect.size.height) * alignment.1,
        );
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_child(&self.child, &self.child_rect);
    }
}

pub const CENTER: Alignment = Alignment(0.5, 0.5);

pub fn center(child: impl Ui) -> impl Ui {
    align(child, CENTER)
}

pub fn align(child: impl Ui, alignment: impl IntoValue<Alignment>) -> impl Ui {
    Align::new(child, alignment.into_value())
}

#[derive(Clone, new)]
pub struct Margin<T: Ui, M: Value<LayoutSideOffsets>> {
    child: T,
    margin: M,
}

#[impl_ui_crate(child)]
impl<T: Ui, M: Value<LayoutSideOffsets>> Margin<T, M> {
    #[Ui]
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut child_sz = self.child.measure(available_size);
        child_sz.width += self.margin.left + self.margin.right;
        child_sz.height += self.margin.top + self.margin.bottom;
        child_sz
    }
    #[Ui]
    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width -= self.margin.left + self.margin.right;
        final_size.height -= self.margin.top + self.margin.bottom;
        self.child.arrange(final_size);
    }
    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        let sz = f.final_size();
        let rect = euclid::rect(
            self.margin.left,
            self.margin.top,
            sz.width - self.margin.left - self.margin.right,
            sz.height - self.margin.top - self.margin.bottom,
        );
        f.push_child(&self.child, &rect);
    }
}

pub fn margin(child: impl Ui, margin: impl IntoValue<LayoutSideOffsets>) -> impl Ui {
    Margin::new(child, margin.into_value())
}
