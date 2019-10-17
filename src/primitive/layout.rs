use crate::core::*;
use webrender::euclid;

/// Constrain a child to a size.
/// # Constructors
/// Can be initialized using [`size(child, size)` function](size) and [`child.size(size)`](ExactSize::size).
#[derive(Clone, new)]
pub struct UiSize<T: Ui> {
    child: T,
    size: LayoutSize,
}

#[impl_ui_crate(child)]
impl<T: Ui> UiSize<T> {
    #[Ui]
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.child.measure(self.size);
        self.size
    }
}

pub fn size<T: Ui>(child: T, size: LayoutSize) -> UiSize<T> {
    UiSize::new(child, size)
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

pub fn width<T: Ui>(child: T, width: LayoutSize) -> UiSize<T> {
    UiSize::new(child, width)
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
pub fn height<T: Ui>(child: T, height: LayoutSize) -> UiSize<T> {
    UiSize::new(child, height)
}
pub trait ExactSize: Ui + Sized {
    fn width(self, width: f32) -> UiWidth<Self> {
        UiWidth::new(self, width)
    }

    fn height(self, height: f32) -> UiHeight<Self> {
        UiHeight::new(self, height)
    }

    fn size(self, size: LayoutSize) -> UiSize<Self> {
        UiSize::new(self, size)
    }

    fn size_wh(self, width: f32, height: f32) -> UiSize<Self> {
        UiSize::new(self, LayoutSize::new(width, height))
    }
}
impl<T: Ui> ExactSize for T {}

#[derive(Clone, new)]
pub struct Center<T: Ui> {
    child: T,
    #[new(default)]
    child_rect: LayoutRect,
}
#[impl_ui_crate(child)]
impl<T: Ui> Center<T> {
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

        self.child_rect.origin = LayoutPoint::new(
            (final_size.width - self.child_rect.size.width) / 2.,
            (final_size.height - self.child_rect.size.height) / 2.,
        );
    }
    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_child(&self.child, &self.child_rect);
    }
}

pub fn center<T: Ui>(child: T) -> Center<T> {
    Center::new(child)
}
pub trait Align: Ui + Sized {
    fn center(self) -> Center<Self> {
        Center::new(self)
    }
}
impl<T: Ui> Align for T {}

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
pub trait MarginExt: Ui + Sized {
    fn margin<M: IntoValue<LayoutSideOffsets>>(self, margin: M) -> Margin<Self, M::Value> {
        Margin::new(self, margin.into_value())
    }
}
impl<T: Ui> MarginExt for T {}

pub fn margin<T: Ui, M: IntoValue<LayoutSideOffsets>>(child: T, margin: M) -> Margin<T, M::Value> {
    Margin::new(child, margin.into_value())
}
