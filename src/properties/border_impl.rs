use crate::core::*;
use crate::{impl_ui_node, property};
pub use wapi::BorderRadius;
use webrender::api as wapi;

impl IntoVar<BorderDetails> for ColorF {
    type Var = OwnedVar<BorderDetails>;

    fn into_var(self) -> Self::Var {
        let border_side = BorderSide {
            color: self,
            style: BorderStyle::Solid,
        };
        OwnedVar(BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        })
    }
}

impl<V: Var<ColorF>> IntoVar<BorderDetails> for V {
    #[allow(clippy::type_complexity)]
    type Var = MapVar<ColorF, Self, BorderDetails, Box<dyn FnMut(&ColorF) -> BorderDetails>>;

    fn into_var(self) -> Self::Var {
        self.map(Box::new(|color: &ColorF| {
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
        }))
    }
}

impl IntoVar<BorderDetails> for (ColorF, BorderStyle) {
    type Var = OwnedVar<BorderDetails>;

    fn into_var(self) -> Self::Var {
        let border_side = BorderSide {
            color: self.0,
            style: self.1,
        };
        OwnedVar(BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        })
    }
}

impl<V: Var<ColorF>> IntoVar<BorderDetails> for (V, BorderStyle) {
    #[allow(clippy::type_complexity)]
    type Var = MapVar<ColorF, V, BorderDetails, Box<dyn FnMut(&ColorF) -> BorderDetails>>;

    fn into_var(self) -> Self::Var {
        let style = self.1;
        self.0.map(Box::new(move |color: &ColorF| {
            let border_side = BorderSide { color: *color, style };
            BorderDetails {
                left: border_side,
                right: border_side,
                top: border_side,
                bottom: border_side,
                radius: BorderRadius::zero(),
            }
        }))
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
impl BorderSide {
    pub fn visible(&self) -> bool {
        self.color.a > 0.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderDetails {
    pub left: BorderSide,
    pub right: BorderSide,
    pub top: BorderSide,
    pub bottom: BorderSide,
    pub radius: BorderRadius,
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
        // SAFETY: wapi::BorderStyle is also repr(u32)
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

struct Border<T: UiNode, L: LocalVar<LayoutSideOffsets>, B: Var<BorderDetails>> {
    child: T,
    widths: L,
    details: B,
    render_details: wapi::BorderDetails,
    child_rect: LayoutRect,
    final_size: LayoutSize,
    visible: bool,
}

#[impl_ui_node(child)]
impl<T: UiNode, L: LocalVar<LayoutSideOffsets>, B: Var<BorderDetails>> UiNode for Border<T, L, B> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);

        let widths = *self.widths.init_local(ctx.vars);
        let details = *self.details.get(ctx.vars);

        self.child_rect.origin = LayoutPoint::new(widths.left, widths.top);
        self.visible = widths.visible() && details.visible();

        self.render_details = details.into();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        let mut visible = false;
        if let Some(&widths) = self.widths.update_local(ctx.vars) {
            visible |= widths.visible();
            self.child_rect.origin = LayoutPoint::new(widths.left, widths.top);
            ctx.updates.push_layout();
        }
        if let Some(&details) = self.details.update(ctx.vars) {
            visible |= details.visible();
            self.render_details = details.into();
            ctx.updates.push_render();
        }
        self.visible = visible;
    }

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        self.child.measure(available_size - self.size_increment()) + self.size_increment()
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.child_rect.size = final_size - self.size_increment();
        self.final_size = final_size;
        self.child.arrange(self.child_rect.size);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if self.visible {
            frame.push_border(
                &LayoutRect::from_size(self.final_size),
                *self.widths.get_local(),
                self.render_details,
            );
        }
        frame.push_ui_node(&self.child, &self.child_rect);
    }
}

impl<T: UiNode, L: LocalVar<LayoutSideOffsets>, B: Var<BorderDetails>> Border<T, L, B> {
    fn size_increment(&self) -> LayoutSize {
        let rw = self.widths.get_local();
        LayoutSize::new(rw.left + rw.right, rw.top + rw.bottom)
    }
}

/// Border property
#[property(outer)]
pub fn border(
    child: impl UiNode,
    widths: impl IntoVar<LayoutSideOffsets>,
    details: impl IntoVar<BorderDetails>,
) -> impl UiNode {
    Border {
        child,
        widths: widths.into_var().as_local(),
        details: details.into_var(),
        render_details: border_details_none(),
        child_rect: LayoutRect::zero(),
        final_size: LayoutSize::zero(),
        visible: false,
    }
}

fn border_details_none() -> wapi::BorderDetails {
    let side_none = wapi::BorderSide {
        color: ColorF::BLACK,
        style: wapi::BorderStyle::None,
    };

    wapi::BorderDetails::Normal(wapi::NormalBorder {
        left: side_none,
        right: side_none,
        top: side_none,
        bottom: side_none,
        radius: {
            wapi::BorderRadius {
                top_left: LayoutSize::zero(),
                top_right: LayoutSize::zero(),
                bottom_left: LayoutSize::zero(),
                bottom_right: LayoutSize::zero(),
            }
        },
        do_aa: true,
    })
}
