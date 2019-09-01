use super::{ColorF, GradientStop, LayoutPoint, LayoutRect, NextFrame, Ui, UiContainer, UiLeaf};

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

impl UiLeaf for FillColor {
    fn render(&self, f: &mut NextFrame) {
        f.push_color(LayoutRect::from_size(f.final_size()), self.color);
    }
}
delegate_ui!(UiLeaf, FillColor);

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

impl UiLeaf for FillGradient {
    fn render(&self, f: &mut NextFrame) {
        let final_size = f.final_size();
        let mut start = self.start;
        let mut end = self.end;

        start.x *= final_size.width;
        start.y *= final_size.height;
        end.x *= final_size.width;
        end.y *= final_size.height;

        f.push_gradient(LayoutRect::from_size(final_size), start, end, self.stops.clone());
    }
}
delegate_ui!(UiLeaf, FillGradient);

pub fn fill_gradient(start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> FillGradient {
    FillGradient::new(start, end, stops)
}

#[derive(Clone)]
pub struct BackgroundColor<T> {
    child: T,
    color: ColorF,
}

impl<T> BackgroundColor<T> {
    pub fn new(child: T, color: ColorF) -> Self {
        BackgroundColor { child, color }
    }
}

impl<T: Ui> UiContainer for BackgroundColor<T> {
    delegate_child!(child, T);

    fn render(&self, f: &mut NextFrame) {
        f.push_color(LayoutRect::from_size(f.final_size()), self.color);
        self.child.render(f)
    }
}
delegate_ui!(UiContainer, BackgroundColor<T>, T);

pub fn background_color<T: Ui>(child: T, color: ColorF) -> BackgroundColor<T> {
    BackgroundColor::new(child, color)
}

#[derive(Clone)]
pub struct BackgroundGradient<T> {
    child: T,
    gradient: FillGradient,
}

impl<T> BackgroundGradient<T> {
    pub fn new(child: T, start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> Self {
        BackgroundGradient {
            child,
            gradient: FillGradient::new(start, end, stops),
        }
    }
}

impl<T: Ui> UiContainer for BackgroundGradient<T> {
    delegate_child!(child, T);

    fn render(&self, f: &mut NextFrame) {
        Ui::render(&self.gradient, f);
        self.child.render(f);
    }
}

delegate_ui!(UiContainer, BackgroundGradient<T>, T);

pub fn background_gradient<T: Ui>(
    child: T,
    start: LayoutPoint,
    end: LayoutPoint,
    stops: Vec<GradientStop>,
) -> BackgroundGradient<T> {
    BackgroundGradient::new(child, start, end, stops)
}

pub trait Background: Ui + Sized {
    fn background_color(self, color: ColorF) -> BackgroundColor<Self> {
        BackgroundColor::new(self, color)
    }

    fn background_gradient(
        self,
        start: LayoutPoint,
        end: LayoutPoint,
        stops: Vec<GradientStop>,
    ) -> BackgroundGradient<Self> {
        BackgroundGradient::new(self, start, end, stops)
    }
}
impl<T: Ui> Background for T {}
