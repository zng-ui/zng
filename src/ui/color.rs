use super::*;

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
pub struct FillGradient<A: Value<LayoutPoint>, B: Value<LayoutPoint>, S: Value<Vec<GradientStop>>> {
    start: A,
    end: B,
    stops: S,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}

#[impl_ui_crate]
impl<A: Value<LayoutPoint>, B: Value<LayoutPoint>, S: Value<Vec<GradientStop>>> FillGradient<A, B, S> {
    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        let final_size = f.final_size();
        let mut start = *self.start;
        let mut end = *self.end;

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

pub fn fill_gradient<A: IntoValue<LayoutPoint>, B: IntoValue<LayoutPoint>, S: IntoValue<Vec<GradientStop>>>(
    start: A,
    end: B,
    stops: S,
) -> FillGradient<A::Value, B::Value, S::Value> {
    FillGradient::new(start.into_value(), end.into_value(), stops.into_value())
}

#[derive(new)]
pub struct Background<T: Ui, B: Ui> {
    child: T,
    background: B,
}

#[impl_ui_crate(child)]
impl<T: Ui, B: Ui> Ui for Background<T, B> {
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let available_size = self.child.measure(available_size);
        self.background.measure(available_size);
        available_size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.background.arrange(final_size);
        self.child.arrange(final_size);
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        self.child.point_over(hits).or_else(|| self.background.point_over(hits))
    }

    fn render(&self, f: &mut NextFrame) {
        self.background.render(f);
        self.child.render(f)
    }
}

pub trait BackgroundExt: Ui + Sized {
    fn background_color<C: IntoValue<ColorF>>(self, color: C) -> Background<Self, FillColor<C::Value>> {
        Background::new(self, fill_color(color))
    }

    fn background_gradient<A: IntoValue<LayoutPoint>, B: IntoValue<LayoutPoint>, S: IntoValue<Vec<GradientStop>>>(
        self,
        start: A,
        end: B,
        stops: S,
    ) -> Background<Self, FillGradient<A::Value, B::Value, S::Value>> {
        Background::new(self, fill_gradient(start, end, stops))
    }
}
impl<T: Ui> BackgroundExt for T {}
