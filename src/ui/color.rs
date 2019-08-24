use super::{ColorF, GradientStop, LayoutPoint, LayoutRect, LayoutSize, RenderContext, Ui};

pub fn rgbf(r: f32, g: f32, b: f32) -> ColorF {
    ColorF::new(r, g, b, 1.)
}

pub fn rgbaf(r: f32, g: f32, b: f32, a: f32) -> ColorF {
    ColorF::new(r, g, b, a)
}

pub fn rgb(r: u8, g: u8, b: u8) -> ColorF {
    ColorF::new(r as f32 / 255., g as f32 / 255., b as f32 / 255., 1.)
}

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> ColorF {
    ColorF::new(r as f32 / 255., g as f32 / 255., b as f32 / 255., a as f32 / 255.)
}

#[derive(Clone)]
pub struct FillColor {
    color: ColorF,
}

impl FillColor {
    pub fn new(color: ColorF) -> Self {
        FillColor { color }
    }
}

#[inline]
fn fill_measure(mut available_size: LayoutSize) -> LayoutSize {
    if available_size.width.is_infinite() {
        available_size.width = 0.;
    }

    if available_size.height.is_infinite() {
        available_size.height = 0.;
    }

    available_size
}

impl Ui for FillColor {
    type Child = ();

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        fill_measure(available_size)
    }

    fn render(&mut self, rc: &mut RenderContext) {
        rc.push_rect(LayoutRect::from_size(rc.final_size()), self.color);
    }
}

pub fn fill_color(color: ColorF) -> FillColor {
    FillColor::new(color)
}

#[derive(Clone)]
pub struct FillGradient {
    start: LayoutPoint,
    end: LayoutPoint,
    stops: Vec<GradientStop>,
}

impl FillGradient {
    pub fn new(start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> Self {
        FillGradient { start, end, stops }
    }
}

impl Ui for FillGradient {
    type Child = ();

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        fill_measure(available_size)
    }

    fn render(&mut self, rc: &mut RenderContext) {
        let final_size = rc.final_size();
        let mut start = self.start;
        let mut end = self.end;

        start.x *= final_size.width;
        start.y *= final_size.height;
        end.x *= final_size.width;
        end.y *= final_size.height;

        rc.push_gradient(LayoutRect::from_size(final_size), start, end, self.stops.clone());
    }
}

pub fn fill_gradient(start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> FillGradient {
    FillGradient::new(start, end, stops)
}

#[derive(Clone)]
pub struct BackgroundColor<T: Ui> {
    child: T,
    color: ColorF,
}

impl<T: Ui> BackgroundColor<T> {
    pub fn new(child: T, color: ColorF) -> Self {
        BackgroundColor { child, color }
    }
}

impl<T: Ui> Ui for BackgroundColor<T> {
    type Child = T;
    fn for_each_child(&mut self, mut action: impl FnMut(&mut Self::Child)) {
        action(&mut self.child);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(available_size)
    }
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size)
    }
    fn render(&mut self, rc: &mut RenderContext) {
        rc.push_rect(LayoutRect::from_size(rc.final_size()), self.color);
        self.child.render(rc)
    }
}
pub fn background_color<T: Ui>(child: T, color: ColorF) -> BackgroundColor<T> {
    BackgroundColor::new(child, color)
}
pub trait BackgroundColorExt: Ui + Sized {
    fn background_color(self, color: ColorF) -> BackgroundColor<Self> {
        BackgroundColor::new(self, color)
    }
}
impl<T: Ui> BackgroundColorExt for T {}
