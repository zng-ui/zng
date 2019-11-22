use crate::core::*;

pub fn rgb<C: Into<ColorFComponent>>(r: C, g: C, b: C) -> ColorF {
    rgba(r, g, b, 1.0)
}

pub fn rgba<C: Into<ColorFComponent>, A: Into<ColorFComponent>>(r: C, g: C, b: C, a: A) -> ColorF {
    ColorF::new(r.into().0, g.into().0, b.into().0, a.into().0)
}

/// `ColorF` component value.
pub struct ColorFComponent(pub f32);

impl From<f32> for ColorFComponent {
    fn from(f: f32) -> Self {
        ColorFComponent(f)
    }
}

impl From<u8> for ColorFComponent {
    fn from(u: u8) -> Self {
        ColorFComponent(f32::from(u) / 255.)
    }
}

impl IntoValue<Vec<GradientStop>> for Vec<(f32, ColorF)> {
    type Value = Owned<Vec<GradientStop>>;

    fn into_value(self) -> Self::Value {
        Owned(
            self.into_iter()
                .map(|(offset, color)| GradientStop { offset, color })
                .collect(),
        )
    }
}

impl IntoValue<Vec<GradientStop>> for Vec<ColorF> {
    type Value = Owned<Vec<GradientStop>>;

    fn into_value(self) -> Self::Value {
        let point = 1. / (self.len() as f32 - 1.);
        Owned(
            self.into_iter()
                .enumerate()
                .map(|(i, color)| GradientStop {
                    offset: (i as f32) * point,
                    color,
                })
                .collect(),
        )
    }
}

#[derive(Clone, new)]
pub struct FillColor<C: Value<ColorF>> {
    color: C,
    #[new(value = "HitTag::new_unique()")]
    hit_tag: HitTag,
}

#[impl_ui_crate]
impl<C: Value<ColorF>> FillColor<C> {
    #[Ui]
    fn value_changed(&mut self, _: &mut UiValues, update: &mut NextUpdate) {
        if self.color.touched() {
            update.render_frame();
        }
    }

    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag)
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        profile_scope!("render_color");
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
    #[new(value = "HitTag::new_unique()")]
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
        profile_scope!("render_gradient");

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
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.background.init(values, update);
        self.child.init(values, update);
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let available_size = self.child.measure(available_size);
        self.background.measure(available_size);
        available_size
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.background.arrange(final_size);
        self.child.arrange(final_size);
    }

    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.background.value_changed(values, update);
        self.child.value_changed(values, update);
    }

    fn parent_value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.background.parent_value_changed(values, update);
        self.child.parent_value_changed(values, update);
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        self.child.point_over(hits).or_else(|| self.background.point_over(hits))
    }

    fn render(&self, f: &mut NextFrame) {
        self.background.render(f);
        self.child.render(f)
    }
}

///Background linear gradient.
/// ## Type arguments
/// * `T`: child type
/// * `A`: line start point
/// * `B`: line end point
/// * `S`: gradient stops
pub type BackgroundGradient<T, A, B, S> = Background<T, FillGradient<A, B, S>>;

pub trait BackgroundExt: Ui + Sized {
    fn background_color<C: IntoValue<ColorF>>(self, color: C) -> Background<Self, FillColor<C::Value>> {
        Background::new(self, fill_color(color))
    }

    fn background_gradient<A: IntoValue<LayoutPoint>, B: IntoValue<LayoutPoint>, S: IntoValue<Vec<GradientStop>>>(
        self,
        start: A,
        end: B,
        stops: S,
    ) -> BackgroundGradient<Self, A::Value, B::Value, S::Value> {
        Background::new(self, fill_gradient(start, end, stops))
    }
}
impl<T: Ui> BackgroundExt for T {}
