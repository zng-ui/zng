use super::{LayoutPoint, LayoutRect, LayoutSize, RenderContext, Ui};
use webrender::euclid;

/// Constrain a child to a size.
/// # Constructors
/// Can be initialized using [`size(child, size)` function](size) and [`child.size(size)`](SizeChildExt::size).
pub struct SizeChild<T: Ui> {
    child: T,
    size: LayoutSize,
}
impl<T: Ui> SizeChild<T> {
    pub fn new(child: T, size: LayoutSize) -> Self {
        SizeChild { child, size }
    }
}
impl<T: Ui> Ui for SizeChild<T> {
    fn measure(&mut self, _: LayoutSize) -> LayoutSize {
        self.child.measure(self.size);
        self.size
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size)
    }
    fn render(&self, c: RenderContext) {
        self.child.render(c)
    }
}
pub fn size<T: Ui>(child: T, size: LayoutSize) -> SizeChild<T> {
    SizeChild::new(child, size)
}
pub trait SizeChildExt: Ui + Sized {
    fn size(self, size: LayoutSize) -> SizeChild<Self> {
        SizeChild::new(self, size)
    }

    fn size_wh(self, width: f32, height: f32) -> SizeChild<Self> {
        SizeChild::new(self, LayoutSize::new(width, height))
    }
}
impl<T: Ui> SizeChildExt for T {}


pub struct WidthChild<T: Ui>  {
    child: T,
    width: f32,
}
impl<T: Ui> WidthChild<T> {
    pub fn new(child: T, width: f32) -> Self {
        WidthChild { child, width }
    }
}
impl<T: Ui> Ui for WidthChild<T> {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.width = self.width;
        let mut child_size = self.child.measure(available_size);
        child_size.width = self.width;
        child_size
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size)
    }
    fn render(&self, c: RenderContext) {
        self.child.render(c)
    }
}
pub fn width<T: Ui>(child: T, width: LayoutSize) -> SizeChild<T> {
    SizeChild::new(child, width)
}
pub trait WidthChildExt: Ui + Sized {
    fn width(self, width: f32) -> WidthChild<Self> {
        WidthChild::new(self, width)
    }
}
impl<T: Ui> WidthChildExt for T {}


pub struct HeightChild<T: Ui>  {
    child: T,
    height: f32,
}
impl<T: Ui> HeightChild<T> {
    pub fn new(child: T, height: f32) -> Self {
        HeightChild { child, height }
    }
}
impl<T: Ui> Ui for HeightChild<T> {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.height = self.height;
        let mut child_size = self.child.measure(available_size);
        child_size.height = self.height;
        child_size
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size)
    }
    fn render(&self, c: RenderContext) {
        self.child.render(c)
    }
}
pub fn height<T: Ui>(child: T, height: LayoutSize) -> SizeChild<T> {
    SizeChild::new(child, height)
}
pub trait HeightChildExt: Ui + Sized {
    fn height(self, height: f32) -> HeightChild<Self> {
        HeightChild::new(self, height)
    }
}
impl<T: Ui> HeightChildExt for T {}


pub struct CenterChild<T: Ui> {
    child: T,
    child_rect: LayoutRect,
}
impl<T: Ui> CenterChild<T> {
    pub fn new(child: T) -> Self {
        CenterChild {
            child,
            child_rect: LayoutRect::default(),
        }
    }
}
impl<T: Ui> Ui for CenterChild<T> {
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
    fn render(&self, mut c: RenderContext) {
        c.push_child(&self.child, &self.child_rect);
    }
}
pub fn center<T: Ui>(child: T) -> CenterChild<T> {
    CenterChild::new(child)
}
pub trait CenterChildExt: Ui + Sized {
    fn center(self) -> CenterChild<Self> {
        CenterChild::new(self)
    }
}
impl<T: Ui> CenterChildExt for T {}

pub struct Margin<T: Ui> {
    child: T,
    left: f32,
    top: f32,
    right: f32,
    bottom: f32,
}
impl<T: Ui> Margin<T> {
    pub fn uniform(child: T, uniform: f32) -> Self {
        Self::ltrb(child, uniform, uniform, uniform, uniform)
    }

    pub fn lr_tb(child: T, left_right: f32, top_bottom: f32) -> Self {
        Self::ltrb(child, left_right, top_bottom, left_right, top_bottom)
    }

    pub fn ltrb(child: T, left: f32, top: f32, right: f32, bottom: f32) -> Self {
        Self {
            child,
            left,
            top,
            right,
            bottom,
        }
    }
}
impl<T: Ui> Ui for Margin<T> {
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
    fn render(&self, mut c: RenderContext) {
        let sz = c.final_size();
        let rect = euclid::rect(
            self.left,
            self.top,
            sz.width - self.left - self.right,
            sz.height - self.top - self.bottom,
        );
        c.push_child(&self.child, &rect);
    }
}
pub trait MarginExt: Ui + Sized {
    fn margin(self, uniform: f32) -> Margin<Self> {
        Margin::uniform(self, uniform)
    }
    fn margin_lr_tb(self, left_right: f32, top_bottom: f32) -> Margin<Self> {
        Margin::lr_tb(self, left_right, top_bottom)
    }
    fn margin_ltrb(self, left: f32, top: f32, right: f32, bottom: f32) -> Margin<Self> {
        Margin::ltrb(self, left, top, right, bottom)
    }
}
impl<T: Ui> MarginExt for T {}

pub fn margin<T: Ui>(child: T, uniform: f32) -> Margin<T> {
    Margin::uniform(child, uniform)
}
pub fn margin_lr_tb<T: Ui>(child: T, left_right: f32, top_bottom: f32) -> Margin<T> {
    Margin::lr_tb(child, left_right, top_bottom)
}
pub fn margin_ltrb<T: Ui>(child: T, left: f32, top: f32, right: f32, bottom: f32) -> Margin<T> {
    Margin::ltrb(child, left, top, right, bottom)
}

struct ListChild {
    child: Box<dyn Ui>,
    rect: LayoutRect,
}

pub struct HorizontalList {
    children: Vec<ListChild>,
}
impl HorizontalList {
    pub fn new(children: Vec<Box<dyn Ui>>) -> Self {
        HorizontalList {
            children: children
                .into_iter()
                .map(|child| ListChild {
                    child,
                    rect: LayoutRect::default(),
                })
                .collect(),
        }
    }
}
impl Ui for HorizontalList {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        let mut total_size = LayoutSize::default();
        
        available_size.width = std::f32::INFINITY;
        for c in self.children.iter_mut() {
            c.rect.size = c.child.measure(available_size);
            total_size.height = total_size.height.max(c.rect.size.height);
            total_size.width += c.rect.size.width;
        }

        total_size
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        let mut x = 0.0;
        for c in self.children.iter_mut() {
            c.rect.origin.x = x;
            c.rect.size.height = c.rect.size.height.min(final_size.height);
            x += c.rect.size.width;
            c.child.arrange(c.rect.size);
        }
    }
    fn render(&self, mut r: RenderContext) {
        for c in self.children.iter() {
            r.push_child(&c.child, &c.rect);
        }
    }
}
pub fn h_list(children: Vec<Box<dyn Ui>>) -> HorizontalList {
    HorizontalList::new(children)
}

pub struct VerticalList {
    children: Vec<ListChild>,
}
impl VerticalList {
    pub fn new(children: Vec<Box<dyn Ui>>) -> Self {
        VerticalList {
            children: children
                .into_iter()
                .map(|child| ListChild {
                    child,
                    rect: LayoutRect::default(),
                })
                .collect(),
        }
    }
}
impl Ui for VerticalList {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        let mut total_size = LayoutSize::default();
        
        available_size.height = std::f32::INFINITY;
        for c in self.children.iter_mut() {
            c.rect.size = c.child.measure(available_size);
            total_size.width = total_size.width.max(c.rect.size.width);
            total_size.height += c.rect.size.height;
        }

        total_size
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        let mut y = 0.0;
        for c in self.children.iter_mut() {
            c.rect.origin.y = y;
            c.rect.size.width = c.rect.size.width.min(final_size.width);
            y += c.rect.size.height;
            c.child.arrange(c.rect.size);
        }
    }
    fn render(&self, mut r: RenderContext) {
        for c in self.children.iter() {
            r.push_child(&c.child, &c.rect);
        }
    }
}
pub fn v_list(children: Vec<Box<dyn Ui>>) -> VerticalList {
    VerticalList::new(children)
}
