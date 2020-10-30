use crate::core::color::{RenderColor, Rgba};
use crate::core::context::*;
use crate::core::render::*;
use crate::core::types::*;
use crate::core::units::*;
use crate::core::var::*;
use crate::core::UiNode;
use crate::core::{impl_ui_node, property};
use webrender::api as w_api;

impl IntoVar<BorderDetails> for Rgba {
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

impl IntoVar<BorderDetails> for (Rgba, BorderStyle) {
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

impl<V: Var<Rgba>> IntoVar<BorderDetails> for (V, BorderStyle) {
    #[allow(clippy::type_complexity)]
    type Var = RcMapVar<Rgba, BorderDetails, V, Box<dyn FnMut(&Rgba) -> BorderDetails>>;

    fn into_var(self) -> Self::Var {
        let style = self.1;
        self.0.map(Box::new(move |color: &Rgba| {
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
    pub color: Rgba,
    pub style: BorderStyle,
}
impl BorderSide {
    #[inline]
    pub fn new(color: Rgba, style: BorderStyle) -> Self {
        BorderSide { color, style }
    }

    #[inline]
    pub fn solid(color: Rgba) -> Self {
        Self::new(color, BorderStyle::Solid)
    }

    #[inline]
    pub fn dotted(color: Rgba) -> Self {
        Self::new(color, BorderStyle::Dotted)
    }

    #[inline]
    pub fn dashed(color: Rgba) -> Self {
        Self::new(color, BorderStyle::Dashed)
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
    #[inline]
    pub fn solid(color: Rgba) -> Self {
        Self::new_all_same(BorderSide::solid(color))
    }

    #[inline]
    pub fn dotted(color: Rgba) -> Self {
        Self::new_all_same(BorderSide::dotted(color))
    }

    #[inline]
    pub fn dashed(color: Rgba) -> Self {
        Self::new_all_same(BorderSide::dashed(color))
    }

    #[inline]
    pub fn new_all_same(side: BorderSide) -> Self {
        BorderDetails {
            left: side,
            right: side,
            top: side,
            bottom: side,
            radius: new_border_radius_all_same_circular(0.0),
        }
    }
}
impl From<Rgba> for BorderDetails {
    fn from(color: Rgba) -> Self {
        BorderDetails::solid(color)
    }
}

pub fn new_border_radius_all_same(corner_radii: LayoutSize) -> BorderRadius {
    BorderRadius {
        top_left: corner_radii,
        top_right: corner_radii,
        bottom_left: corner_radii,
        bottom_right: corner_radii,
    }
}

pub fn new_border_radius_all_same_circular(corner_radius: f32) -> BorderRadius {
    new_border_radius_all_same(LayoutSize::new(corner_radius, corner_radius))
}

trait VisibleExt {
    fn visible(&self) -> bool;
}
impl VisibleExt for LayoutSideOffsets {
    fn visible(&self) -> bool {
        self.top > 0.0 || self.bottom > 0.0 || self.left > 0.0 || self.right > 0.0
    }
}
impl VisibleExt for w_api::BorderDetails {
    fn visible(&self) -> bool {
        match self {
            w_api::BorderDetails::Normal(border) => border.visible(),
            w_api::BorderDetails::NinePatch(_) => unimplemented!(),
        }
    }
}
impl VisibleExt for w_api::NormalBorder {
    fn visible(&self) -> bool {
        self.left.visible() || self.right.visible() || self.top.visible() || self.bottom.visible()
    }
}
impl VisibleExt for w_api::BorderSide {
    fn visible(&self) -> bool {
        !self.style.is_hidden() && self.color.a > f32::EPSILON
    }
}

impl From<BorderStyle> for w_api::BorderStyle {
    fn from(border_style: BorderStyle) -> Self {
        // SAFETY: w_api::BorderStyle is also repr(u32)
        // and contains all values
        unsafe { std::mem::transmute(border_style) }
    }
}
impl From<BorderSide> for w_api::BorderSide {
    fn from(border_side: BorderSide) -> Self {
        w_api::BorderSide {
            color: border_side.color.into(),
            style: border_side.style.into(),
        }
    }
}
impl From<BorderDetails> for w_api::BorderDetails {
    fn from(border_details: BorderDetails) -> Self {
        w_api::BorderDetails::Normal(w_api::NormalBorder {
            left: border_details.left.into(),
            right: border_details.right.into(),
            top: border_details.top.into(),
            bottom: border_details.bottom.into(),
            radius: border_details.radius,
            do_aa: true,
        })
    }
}

struct BorderNode<T: UiNode, L: VarLocal<SideOffsets>, B: Var<BorderDetails>> {
    child: T,

    widths: L,
    details: B,
    child_rect: LayoutRect,

    final_widths: LayoutSideOffsets,
    final_size: LayoutSize,
    final_details: w_api::BorderDetails,

    visible: bool,
}

#[impl_ui_node(child)]
impl<T: UiNode, L: VarLocal<SideOffsets>, B: Var<BorderDetails>> UiNode for BorderNode<T, L, B> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        self.child.init(ctx);

        self.widths.init_local(ctx.vars);
        let details = *self.details.get(ctx.vars);
        self.final_details = details.into();
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        self.child.update(ctx);

        if self.widths.update_local(ctx.vars).is_some() {
            ctx.updates.push_layout()
        }

        if let Some(&details) = self.details.get_new(ctx.vars) {
            self.final_details = details.into();
            ctx.updates.push_render()
        }
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        self.final_widths = self.widths.get_local().to_layout(available_size, ctx);

        self.visible = self.final_widths.visible() && self.final_details.visible();

        let size_inc = self.size_increment();
        self.child.measure(available_size - size_inc, ctx) + size_inc
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        self.child_rect.origin = LayoutPoint::new(self.final_widths.left, self.final_widths.top);
        self.child_rect.size = final_size - self.size_increment();
        self.final_size = final_size;
        self.child.arrange(self.child_rect.size, ctx);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if self.visible {
            frame.push_border(LayoutRect::from_size(self.final_size), self.final_widths, self.final_details);
        }
        frame.push_reference_frame(self.child_rect.origin, |frame| self.child.render(frame));
    }
}

impl<T: UiNode, L: VarLocal<SideOffsets>, B: Var<BorderDetails>> BorderNode<T, L, B> {
    fn size_increment(&self) -> LayoutSize {
        let rw = self.final_widths;
        LayoutSize::new(rw.left + rw.right, rw.top + rw.bottom)
    }
}

/// Border property
#[property(inner)]
pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, details: impl IntoVar<BorderDetails>) -> impl UiNode {
    BorderNode {
        child,

        widths: widths.into_local(),
        details: details.into_var(),

        child_rect: LayoutRect::zero(),
        final_details: border_details_none(),
        final_size: LayoutSize::zero(),
        final_widths: LayoutSideOffsets::zero(),

        visible: false,
    }
}

fn border_details_none() -> w_api::BorderDetails {
    let side_none = w_api::BorderSide {
        color: RenderColor::BLACK,
        style: w_api::BorderStyle::None,
    };

    w_api::BorderDetails::Normal(w_api::NormalBorder {
        left: side_none,
        right: side_none,
        top: side_none,
        bottom: side_none,
        radius: {
            w_api::BorderRadius {
                top_left: LayoutSize::zero(),
                top_right: LayoutSize::zero(),
                bottom_left: LayoutSize::zero(),
                bottom_right: LayoutSize::zero(),
            }
        },
        do_aa: true,
    })
}
