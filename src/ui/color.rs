use super::{
    ColorF, GradientStop, HitTag, Hits, IntoValue, LayoutPoint, LayoutRect, NextFrame, NextUpdate, Ui, UiContainer,
    UiLeaf, UiValues, Value,
};

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

#[derive(Clone, new)]
pub struct FillColor {
    color: ColorF,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}

impl UiLeaf for FillColor {
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    fn render(&self, f: &mut NextFrame) {
        f.push_color(LayoutRect::from_size(f.final_size()), self.color, Some(self.hit_tag));
    }
}
delegate_ui!(UiLeaf, FillColor);
#[cfg(test)]
mod fill_color_tests {
    use super::*;

    ui_leaf_tests!(FillColor::new(rgb(0, 0, 0)));
}

pub fn fill_color(color: ColorF) -> FillColor {
    FillColor::new(color)
}

#[derive(Clone, new)]
pub struct FillGradient {
    start: LayoutPoint,
    end: LayoutPoint,
    stops: Vec<GradientStop>,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}
impl UiLeaf for FillGradient {
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    fn render(&self, f: &mut NextFrame) {
        let final_size = f.final_size();
        let mut start = self.start;
        let mut end = self.end;

        start.x *= final_size.width;
        start.y *= final_size.height;
        end.x *= final_size.width;
        end.y *= final_size.height;

        f.push_gradient(
            LayoutRect::from_size(final_size),
            start,
            end,
            self.stops.clone(),
            Some(self.hit_tag),
        );
    }
}
delegate_ui!(UiLeaf, FillGradient);
#[cfg(test)]
mod fill_gradient_tests {
    use super::*;

    ui_leaf_tests!(FillGradient::new(
        LayoutPoint::new(0., 0.),
        LayoutPoint::new(1., 1.),
        vec![
            GradientStop {
                offset: 0.,
                color: rgb(0, 200, 0),
            },
            GradientStop {
                offset: 1.,
                color: rgb(200, 0, 0),
            },
        ]
    ));
}

pub fn fill_gradient(start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> FillGradient {
    FillGradient::new(start, end, stops)
}

#[derive(Clone, new)]
pub struct BackgroundColor<T: Ui, C: Value<ColorF>> {
    child: T,
    color: C,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}

impl<T: Ui, C: Value<ColorF>> UiContainer for BackgroundColor<T, C> {
    delegate_child!(child, T);

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.value_changed(values, update);
        if self.color.changed() {
            update.render_frame();
        }
    }

    fn render(&self, f: &mut NextFrame) {
        f.push_color(LayoutRect::from_size(f.final_size()), *self.color, Some(self.hit_tag));
        self.child.render(f)
    }
}
impl<T: Ui, C: Value<ColorF>> Ui for BackgroundColor<T, C> {
    delegate_ui_methods!(UiContainer);
}

pub fn background_color<T: Ui, C: Value<ColorF>>(child: T, color: C) -> BackgroundColor<T, C> {
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

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        Ui::point_over(&self.gradient, hits)
    }

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
    fn background_color<C: IntoValue<ColorF>>(self, color: C) -> BackgroundColor<Self, C::Value> {
        BackgroundColor::new(self, color.into_value())
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

#[cfg(test)]
mod background_color_tests {
    use super::*;

    ui_container_tests!(|c: TestChild| c.background_color(rgb(0, 0, 0)));
}

#[cfg(test)]
mod background_gradient_tests {
    use super::*;

    ui_container_tests!(|c: TestChild| c.background_gradient(
        LayoutPoint::new(0., 0.),
        LayoutPoint::new(1., 1.),
        vec![
            GradientStop {
                offset: 0.,
                color: rgb(0, 200, 0),
            },
            GradientStop {
                offset: 1.,
                color: rgb(200, 0, 0),
            },
        ]
    ));
}
