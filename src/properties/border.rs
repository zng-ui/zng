//! Border property and types.

use crate::prelude::new_property::*;
use webrender_api as w_api;

pub use w_api::BorderRadius;

impl_from_and_into_var! {
    /// All sides solid style, same `self` color. Square corners.
    fn from(color: Rgba) -> BorderDetails {
        let border_side = BorderSide {
            color,
            style: BorderStyle::Solid,
        };
        BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        }
    }

    /// All sides solid style, first color applied to top and bottom,
    /// second color applied to left and right. Square corners
    fn from((top_bottom, left_right): (Rgba, Rgba)) -> BorderDetails {
        let top_bottom = BorderSide {
            color: top_bottom,
            style: BorderStyle::Solid,
        };
        let left_right = BorderSide {
            color: left_right,
            style: BorderStyle::Solid,
        };
        BorderDetails {
            left: left_right,
            right: left_right,
            top: top_bottom,
            bottom: top_bottom,
            radius: BorderRadius::zero(),
        }
    }

    /// Each side a color in order, top, right, bottom, left. All sides solid style. Square corners.
    fn from((top, right, bottom, left): (Rgba, Rgba, Rgba, Rgba)) -> BorderDetails {
        BorderDetails {
            top: BorderSide {
                color: top,
                style: BorderStyle::Solid,
            },
            right: BorderSide {
                color: right,
                style: BorderStyle::Solid,
            },
            bottom: BorderSide {
                color: bottom,
                style: BorderStyle::Solid,
            },
            left: BorderSide {
                color: left,
                style: BorderStyle::Solid,
            },
            radius: BorderRadius::zero()
        }
    }

     /// All sides same color and style. Square corners.
    fn from((color, style): (Rgba, BorderStyle)) -> BorderDetails {
        let border_side = BorderSide {
            color,
            style,
        };
        BorderDetails {
            left: border_side,
            right: border_side,
            top: border_side,
            bottom: border_side,
            radius: BorderRadius::zero(),
        }
    }

    /// All sides the same style, first color applied to top and bottom,
    /// second color applied to left and right. Square corners.
    fn from((top_bottom, left_right, style): (Rgba, Rgba, BorderStyle)) -> BorderDetails {
        let top_bottom = BorderSide {
            color: top_bottom,
            style,
        };
        let left_right = BorderSide {
            color: left_right,
            style,
        };
        BorderDetails {
            left: left_right,
            right: left_right,
            top: top_bottom,
            bottom: top_bottom,
            radius: BorderRadius::zero(),
        }
    }

    /// Each side a color in order, top, right, bottom, left. All sides the same style. Square corners.
    fn from((top, right, bottom, left, style): (Rgba, Rgba, Rgba, Rgba, BorderStyle)) -> BorderDetails {
        BorderDetails {
            top: BorderSide {
                color: top,
                style,
            },
            right: BorderSide {
                color: right,
                style,
            },
            bottom: BorderSide {
                color: bottom,
                style,
            },
            left: BorderSide {
                color: left,
                style,
            },
            radius: BorderRadius::zero()
        }
    }

    /// First color and style applied to top and bottom,
    /// second color and style applied to left and right. Square corners.
    fn from((top_bottom_color, top_bottom_style, left_right_color, left_right_style): (Rgba, BorderStyle, Rgba, BorderStyle)) -> BorderDetails {
        let top_bottom = BorderSide {
            color: top_bottom_color,
            style: top_bottom_style,
        };
        let left_right = BorderSide {
            color: left_right_color,
            style: left_right_style,
        };
        BorderDetails {
            left: left_right,
            right: left_right,
            top: top_bottom,
            bottom: top_bottom,
            radius: BorderRadius::zero(),
        }
    }

    /// Each side a color and style in order, top, right, bottom, left. Square corners.
    fn from((top_color, top_style, right_color, right_style, bottom_color, bottom_style, left_color, left_style)
    : (Rgba, BorderStyle, Rgba, BorderStyle, Rgba, BorderStyle, Rgba, BorderStyle)) -> BorderDetails {
        BorderDetails {
            top: BorderSide {
                color: top_color,
                style: top_style,
            },
            right: BorderSide {
                color: right_color,
                style: right_style,
            },
            bottom: BorderSide {
                color: bottom_color,
                style: bottom_style,
            },
            left: BorderSide {
                color: left_color,
                style: left_style,
            },
            radius: BorderRadius::zero()
        }
    }
}

/// The line style for the sides of a widget's border.
#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Hash, Eq)]
pub enum BorderStyle {
    /// A single straight solid line.
    Solid = 1,
    /// Two straight solid lines that add up to the pixel size defined by the side width.
    Double = 2,

    /// Displays a series of rounded dots.
    Dotted = 3,
    /// Displays a series of short square-ended dashes or line segments.
    Dashed = 4,

    /// Fully transparent line.
    Hidden = 5,

    /// Displays a border with a carved appearance.
    Groove = 6,
    /// Displays a border with an extruded appearance.
    Ridge = 7,

    /// Displays a border that makes the widget appear embedded.
    Inset = 8,
    /// Displays a border that makes the widget appear embossed.
    Outset = 9,
}

/// The line style and color for the sides of a widget's border.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderSide {
    /// Line color.
    pub color: Rgba,
    /// Line style.
    pub style: BorderStyle,
}
impl BorderSide {
    /// New border side from color and style value.
    pub fn new<C: Into<Rgba>, S: Into<BorderStyle>>(color: C, style: S) -> Self {
        BorderSide {
            color: color.into(),
            style: style.into(),
        }
    }

    /// New border side with [`Solid`](BorderStyle::Solid) style.
    pub fn solid<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Solid)
    }
    /// New border side with [`Double`](BorderStyle::Double) style.
    pub fn double<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Double)
    }

    /// New border side with [`Solid`](BorderStyle::Dotted) style.
    pub fn dotted<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Dotted)
    }
    /// New border side with [`Solid`](BorderStyle::Dashed) style.
    pub fn dashed<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Dashed)
    }

    /// New border side with [`Groove`](BorderStyle::Groove) style.
    pub fn groove<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Groove)
    }
    /// New border side with [`Ridge`](BorderStyle::Ridge) style.
    pub fn ridge<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Ridge)
    }

    /// New border side with [`Inset`](BorderStyle::Inset) style.
    pub fn inset<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Inset)
    }

    /// New border side with [`Outset`](BorderStyle::Outset) style.
    pub fn outset<C: Into<Rgba>>(color: C) -> Self {
        Self::new(color, BorderStyle::Outset)
    }

    /// New border side with [`Hidden`](BorderStyle::Hidden) style and transparent color.
    #[inline]
    pub fn hidden() -> Self {
        Self::new(colors::BLACK.transparent(), BorderStyle::Hidden)
    }
}

/// The line style and color for each side of a widget's border, plus the radius of each corner.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderDetails {
    /// Color and style of the left border.
    pub left: BorderSide,
    /// Color and style of the right border.
    pub right: BorderSide,

    /// Color and style of the top border.
    pub top: BorderSide,
    /// Color and style of the bottom border.
    pub bottom: BorderSide,

    /// Corner radius of each corner.
    /// TODO, length based radius.
    pub radius: BorderRadius,
}
impl BorderDetails {
    /// All sides equal and square corners.
    pub fn new_all<S: Into<BorderSide>>(side: S) -> Self {
        let side = side.into();
        BorderDetails {
            left: side,
            right: side,
            top: side,
            bottom: side,
            radius: new_border_radius_all_same_circular(0.0),
        }
    }

    /// Top-bottom and left-right equal and square corners.
    pub fn new_dimension<TB: Into<BorderSide>, LR: Into<BorderSide>>(top_bottom: TB, left_right: LR) -> Self {
        let top_bottom = top_bottom.into();
        let left_right = left_right.into();
        BorderDetails {
            left: left_right,
            right: left_right,
            top: top_bottom,
            bottom: top_bottom,
            radius: new_border_radius_all_same_circular(0.0),
        }
    }
    /// New top, right, bottom left and square corners.
    pub fn new<T: Into<BorderSide>, R: Into<BorderSide>, B: Into<BorderSide>, L: Into<BorderSide>>(
        top: T,
        right: R,
        bottom: B,
        left: L,
    ) -> Self {
        BorderDetails {
            left: left.into(),
            right: right.into(),
            top: top.into(),
            bottom: bottom.into(),
            radius: new_border_radius_all_same_circular(0.0),
        }
    }

    /// All sides a solid color.
    pub fn solid<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::solid(color))
    }
    /// All sides a double line solid color.
    pub fn double<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::double(color))
    }

    /// All sides a dotted color.
    pub fn dotted<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::dotted(color))
    }
    /// All sides a dashed color.
    pub fn dashed<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::dashed(color))
    }

    /// All sides a grooved color.
    pub fn groove<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::groove(color))
    }
    /// All sides a ridged color.
    pub fn ridge<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::ridge(color))
    }

    /// All sides a inset color.
    pub fn inset<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::inset(color))
    }
    /// All sides a outset color.
    pub fn outset<C: Into<Rgba>>(color: C) -> Self {
        Self::new_all(BorderSide::outset(color))
    }

    /// All sides hidden.
    #[inline]
    pub fn hidden() -> Self {
        Self::new_all(BorderSide::hidden())
    }
}

fn new_border_radius_all_same(corner_radii: LayoutSize) -> BorderRadius {
    BorderRadius {
        top_left: corner_radii,
        top_right: corner_radii,
        bottom_left: corner_radii,
        bottom_right: corner_radii,
    }
}

fn new_border_radius_all_same_circular(corner_radius: f32) -> BorderRadius {
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

/// Border property
#[property(inner)]
pub fn border(child: impl UiNode, widths: impl IntoVar<SideOffsets>, details: impl IntoVar<BorderDetails>) -> impl UiNode {
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
                ctx.updates.layout()
            }

            if let Some(&details) = self.details.get_new(ctx.vars) {
                self.final_details = details.into();
                ctx.updates.render()
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
