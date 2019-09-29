use super::{
    impl_ui_crate, ColorF, GradientStop, HitTag, Hits, IntoValue, LayoutPoint, LayoutRect, NextFrame, NextUpdate, Ui,
    UiValues, Value,
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
pub struct FillColor<C: Value<ColorF>> {
    color: C,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}

#[impl_ui_crate]
impl<C: Value<ColorF>> FillColor<C> {
    #[Ui]
    fn value_changed(&mut self, _: &mut UiValues, update: &mut NextUpdate) {
        if self.color.changed() {
            update.render_frame();
        }
    }

    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_color(LayoutRect::from_size(f.final_size()), *self.color, Some(self.hit_tag));
    }
}

pub fn fill_color<C: IntoValue<ColorF>>(color: C) -> FillColor<C::Value> {
    FillColor::new(color.into_value())
}

#[derive(Clone, new)]
pub struct FillGradient {
    start: LayoutPoint,
    end: LayoutPoint,
    stops: Vec<GradientStop>,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}

#[impl_ui_crate]
impl FillGradient {
    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    #[Ui]
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

#[impl_ui_crate(child)]
impl<T: Ui, C: Value<ColorF>> BackgroundColor<T, C> {
    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    #[Ui]
    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.value_changed(values, update);
        if self.color.changed() {
            update.render_frame();
        }
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        f.push_color(LayoutRect::from_size(f.final_size()), *self.color, Some(self.hit_tag));
        self.child.render(f)
    }
}

pub fn background_color<T: Ui, C: Value<ColorF>>(child: T, color: C) -> BackgroundColor<T, C> {
    BackgroundColor::new(child, color)
}

#[derive(Clone)]
pub struct BackgroundGradient<T> {
    child: T,
    gradient: FillGradient,
}

#[impl_ui_crate(child)]
impl<T: Ui> BackgroundGradient<T> {
    pub fn new(child: T, start: LayoutPoint, end: LayoutPoint, stops: Vec<GradientStop>) -> Self {
        BackgroundGradient {
            child,
            gradient: FillGradient::new(start, end, stops),
        }
    }

    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        Ui::point_over(&self.gradient, hits)
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        Ui::render(&self.gradient, f);
        self.child.render(f);
    }
}

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
