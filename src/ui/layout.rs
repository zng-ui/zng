use super::{LayoutPoint, LayoutRect, LayoutSize, NextFrame, Ui, UiContainer};
use webrender::euclid;

/// Constrain a child to a size.
/// # Constructors
/// Can be initialized using [`size(child, size)` function](size) and [`child.size(size)`](ExactSize::size).
#[derive(Clone)]
pub struct UiSize<T: Ui> {
    child: T,
    size: LayoutSize,
}

impl<T: Ui> UiSize<T> {
    pub fn new(child: T, size: LayoutSize) -> Self {
        UiSize { child, size }
    }
}

impl<T: Ui> UiContainer for UiSize<T> {
    delegate_child!(child, T);

    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.child.measure(self.size);
        self.size
    }
}
delegate_ui!(UiContainer, UiSize<T>, T);

pub fn size<T: Ui>(child: T, size: LayoutSize) -> UiSize<T> {
    UiSize::new(child, size)
}

#[derive(Clone)]
pub struct UiWidth<T: Ui> {
    child: T,
    width: f32,
}
impl<T: Ui> UiWidth<T> {
    pub fn new(child: T, width: f32) -> Self {
        UiWidth { child, width }
    }
}
impl<T: Ui> UiContainer for UiWidth<T> {
    delegate_child!(child, T);

    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.width = self.width;
        let mut child_size = self.child.measure(available_size);
        child_size.width = self.width;
        child_size
    }
}
delegate_ui!(UiContainer, UiWidth<T>, T);

pub fn width<T: Ui>(child: T, width: LayoutSize) -> UiSize<T> {
    UiSize::new(child, width)
}

#[derive(Clone)]
pub struct UiHeight<T: Ui> {
    child: T,
    height: f32,
}
impl<T: Ui> UiHeight<T> {
    pub fn new(child: T, height: f32) -> Self {
        UiHeight { child, height }
    }
}
impl<T: Ui> UiContainer for UiHeight<T> {
    delegate_child!(child, T);

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

#[derive(Clone)]
pub struct Center<T: Ui> {
    child: T,
    child_rect: LayoutRect,
}
impl<T: Ui> Center<T> {
    pub fn new(child: T) -> Self {
        Center {
            child,
            child_rect: LayoutRect::default(),
        }
    }
}
impl<T: Ui> UiContainer for Center<T> {
    delegate_child!(child, T);

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
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child_rect.size = self.child_rect.size.min(final_size);
        self.child.arrange(self.child_rect.size);

        self.child_rect.origin = LayoutPoint::new(
            (final_size.width - self.child_rect.size.width) / 2.,
            (final_size.height - self.child_rect.size.height) / 2.,
        );
    }
    fn render(&self, f: &mut NextFrame) {
        f.push_child(&self.child, &self.child_rect);
    }
}
delegate_ui!(UiContainer, Center<T>, T);

pub fn center<T: Ui>(child: T) -> Center<T> {
    Center::new(child)
}
pub trait Align: Ui + Sized {
    fn center(self) -> Center<Self> {
        Center::new(self)
    }
}
impl<T: Ui> Align for T {}

#[derive(Clone)]
pub struct UiMargin<T: Ui> {
    child: T,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}
impl<T: Ui> UiMargin<T> {
    pub fn uniform(child: T, uniform: f32) -> Self {
        Self::ltrb(child, uniform, uniform, uniform, uniform)
    }

    pub fn lr_tb(child: T, left_right: f32, top_bottom: f32) -> Self {
        Self::ltrb(child, left_right, top_bottom, left_right, top_bottom)
    }

    pub fn ltrb(child: T, left: f32, top: f32, right: f32, bottom: f32) -> Self {
        UiMargin {
            child,
            left,
            top,
            right,
            bottom,
        }
    }
}
impl<T: Ui> UiContainer for UiMargin<T> {
    delegate_child!(child, T);

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let mut child_sz = self.child.measure(available_size);
        child_sz.width += self.left + self.right;
        child_sz.height += self.top + self.bottom;
        child_sz
    }
    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width -= self.left + self.right;
        final_size.height -= self.top + self.bottom;
        self.child.arrange(final_size);
    }
    fn render(&self, f: &mut NextFrame) {
        let sz = f.final_size();
        let rect = euclid::rect(
            self.left,
            self.top,
            sz.width - self.left - self.right,
            sz.height - self.top - self.bottom,
        );
        f.push_child(&self.child, &rect);
    }
}
delegate_ui!(UiContainer, UiMargin<T>, T);

pub trait Margin: Ui + Sized {
    fn margin(self, uniform: f32) -> UiMargin<Self> {
        UiMargin::uniform(self, uniform)
    }
    fn margin_lr_tb(self, left_right: f32, top_bottom: f32) -> UiMargin<Self> {
        UiMargin::lr_tb(self, left_right, top_bottom)
    }
    fn margin_ltrb(self, left: f32, top: f32, right: f32, bottom: f32) -> UiMargin<Self> {
        UiMargin::ltrb(self, left, top, right, bottom)
    }
}
impl<T: Ui> Margin for T {}

pub fn margin<T: Ui>(child: T, uniform: f32) -> UiMargin<T> {
    UiMargin::uniform(child, uniform)
}
pub fn margin_lr_tb<T: Ui>(child: T, left_right: f32, top_bottom: f32) -> UiMargin<T> {
    UiMargin::lr_tb(child, left_right, top_bottom)
}
pub fn margin_ltrb<T: Ui>(child: T, left: f32, top: f32, right: f32, bottom: f32) -> UiMargin<T> {
    UiMargin::ltrb(child, left, top, right, bottom)
}
