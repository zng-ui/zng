use super::*;
use webrender::api as wapi;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum BorderStyle {
    Solid = 1,
    Double = 2,
    Dotted = 3,
    Dashed = 4,
    Groove = 6,
    Ridge = 7,
    Inset = 8,
    Outset = 9,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSide {
    pub color: ColorF,
    pub style: BorderStyle,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderDetails {
    pub left: BorderSide,
    pub right: BorderSide,
    pub top: BorderSide,
    pub bottom: BorderSide,
    pub radius: BorderRadius,
}

impl From<BorderStyle> for wapi::BorderStyle {
    fn from(border_style: BorderStyle) -> Self {
        // SAFETY: WBorderStyle is also repr(u32)
        // and contains all values
        unsafe { std::mem::transmute(border_style) }
    }
}
impl From<BorderSide> for wapi::BorderSide {
    fn from(border_side: BorderSide) -> Self {
        wapi::BorderSide {
            color: border_side.color,
            style: border_side.style.into(),
        }
    }
}
impl From<BorderDetails> for wapi::BorderDetails {
    fn from(border_details: BorderDetails) -> Self {
        wapi::BorderDetails::Normal(wapi::NormalBorder {
            left: border_details.left.into(),
            right: border_details.right.into(),
            top: border_details.top.into(),
            bottom: border_details.bottom.into(),
            radius: border_details.radius,
            do_aa: true,
        })
    }
}

#[derive(Clone, new)]
pub struct Border<T: Ui, L: Value<LayoutSideOffsets>, B: Value<BorderDetails>> {
    child: T,
    widths: L,
    details: B,
    #[new(value = "HitTag::new()")]
    hit_tag: HitTag,
}

#[impl_ui_crate(child)]
impl<T: Ui, L: Value<LayoutSideOffsets>, B: Value<BorderDetails>> Ui for Border<T, L, B> {
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.width -= self.widths.left + self.widths.right;
        available_size.height -= self.widths.top + self.widths.bottom;

        let mut result = self.child.measure(available_size);
        result.width += self.widths.left + self.widths.right;
        result.height += self.widths.top + self.widths.bottom;

        result
    }

    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width -= self.widths.left + self.widths.right;
        final_size.height -= self.widths.top + self.widths.bottom;

        self.child.arrange(final_size)
    }

    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        if self.widths.changed() {
            update.update_layout();
        }

        if self.details.changed() {
            update.render_frame();
        }

        self.child.value_changed(values, update);
    }

    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag).or_else(|| {
            self.child.point_over(hits).map(|mut lp| {
                lp.x += self.widths.left;
                lp.y += self.widths.top;
                lp
            })
        })
    }

    fn render(&self, f: &mut NextFrame) {
        let offset = LayoutPoint::new(self.widths.left, self.widths.top);
        let mut size = f.final_size();
        size.width -= self.widths.left + self.widths.right;
        size.height -= self.widths.top + self.widths.bottom;

        f.push_child(&self.child, &LayoutRect::new(offset, size));

        f.push_border(
            LayoutRect::from_size(f.final_size()),
            *self.widths,
            (*self.details).into(),
            Some(self.hit_tag),
        );
    }
}

pub trait BorderExtt: Ui + Sized {
    fn border<L: IntoValue<LayoutSideOffsets>, B: IntoValue<BorderDetails>>(
        self,
        widths: L,
        details: B,
    ) -> Border<Self, L::Value, B::Value> {
        Border::new(self, widths.into_value(), details.into_value())
    }
}
impl<T: Ui> BorderExtt for T {}
