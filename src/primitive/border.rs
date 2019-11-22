use crate::core::*;
pub use wapi::BorderRadius;
use webrender::api as wapi;

impl IntoValue<BorderDetails> for ColorF {
    type Value = Owned<BorderDetails>;

    fn into_value(self) -> Self::Value {
        let border_side = BorderSide {
            color: self,
            style: BorderStyle::Solid,
        };
        Owned(BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        })
    }
}

impl IntoValue<BorderDetails> for Var<ColorF> {
    type Value = Var<BorderDetails>;

    fn into_value(self) -> Self::Value {
        self.map(|color: &ColorF| {
            let border_side = BorderSide {
                color: *color,
                style: BorderStyle::Solid,
            };
            BorderDetails {
                left: border_side,
                right: border_side,
                top: border_side,
                bottom: border_side,
                radius: BorderRadius::zero(),
            }
        })
    }
}

impl IntoValue<BorderDetails> for (ColorF, BorderStyle) {
    type Value = Owned<BorderDetails>;

    fn into_value(self) -> Self::Value {
        let border_side = BorderSide {
            color: self.0,
            style: self.1,
        };
        Owned(BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        })
    }
}

impl IntoValue<BorderDetails> for (Var<ColorF>, BorderStyle) {
    type Value = Var<BorderDetails>;

    fn into_value(self) -> Self::Value {
        let style = self.1;
        self.0.map(move |color: &ColorF| {
            let border_side = BorderSide { color: *color, style };
            BorderDetails {
                left: border_side,
                right: border_side,
                top: border_side,
                bottom: border_side,
                radius: BorderRadius::zero(),
            }
        })
    }
}

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

impl BorderSide {
    pub fn visible(&self) -> bool {
        self.color.a > 0.0
    }
}

impl BorderDetails {
    pub fn visible(&self) -> bool {
        self.left.visible() || self.right.visible() || self.top.visible() || self.bottom.visible()
    }
}

trait LayoutSideOffsetsExt {
    fn visible(&self) -> bool;
}

impl LayoutSideOffsetsExt for LayoutSideOffsets {
    fn visible(&self) -> bool {
        self.top > 0.0 || self.bottom > 0.0 || self.left > 0.0 || self.right > 0.0
    }
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
    #[new(value = "HitTag::new_unique()")]
    hit_tag: HitTag,
    #[new(default)]
    visible: bool,
}

#[impl_ui_crate(child)]
impl<T: Ui, L: Value<LayoutSideOffsets>, B: Value<BorderDetails>> Border<T, L, B> {
    #[Ui]
    fn init(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        self.child.init(values, update);
        self.update_visible(update);
    }

    #[Ui]
    fn measure(&mut self, mut available_size: LayoutSize) -> LayoutSize {
        available_size.width -= self.widths.left + self.widths.right;
        available_size.height -= self.widths.top + self.widths.bottom;

        let mut result = self.child.measure(available_size);
        result.width += self.widths.left + self.widths.right;
        result.height += self.widths.top + self.widths.bottom;

        result
    }

    #[Ui]
    fn arrange(&mut self, mut final_size: LayoutSize) {
        final_size.width -= self.widths.left + self.widths.right;
        final_size.height -= self.widths.top + self.widths.bottom;

        self.child.arrange(final_size)
    }

    #[Ui]
    fn value_changed(&mut self, values: &mut UiValues, update: &mut NextUpdate) {
        let mut update_visible = false;

        if self.widths.touched() {
            update.update_layout();
            update_visible = true;
        }

        if self.details.touched() {
            update.render_frame();
            update_visible = true;
        }

        if update_visible {
            self.update_visible(update);
        }

        self.child.value_changed(values, update);
    }

    #[Ui]
    fn point_over(&self, hits: &Hits) -> Option<LayoutPoint> {
        hits.point_over(self.hit_tag).or_else(|| {
            self.child.point_over(hits).map(|mut lp| {
                lp.x += self.widths.left;
                lp.y += self.widths.top;
                lp
            })
        })
    }

    fn update_visible(&mut self, update: &mut NextUpdate) {
        let visible = self.details.visible() && self.widths.visible();

        if self.visible != visible {
            self.visible = visible;
            update.render_frame();
        }
    }

    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        let final_rect = {
            profile_scope!("render_border");

            let offset = LayoutPoint::new(self.widths.left, self.widths.top);
            let mut size = f.final_size();
            size.width -= self.widths.left + self.widths.right;
            size.height -= self.widths.top + self.widths.bottom;

            if self.visible {
                //border hit_test covers entire area, so if we want to draw the border over the child,
                //it cannot have a hit_tag and transparent hit areas must be drawn for each border segment
                f.push_border(
                    LayoutRect::from_size(f.final_size()),
                    *self.widths,
                    (*self.details).into(),
                    Some(self.hit_tag),
                );
            }

            LayoutRect::new(offset, size)
        };

        f.push_child(&self.child, &final_rect);
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
