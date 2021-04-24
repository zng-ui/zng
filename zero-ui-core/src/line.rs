//! Line and border types.

use webrender::api as w_api;

use crate::{
    color::{colors, RenderColor, Rgba},
    context::LayoutContext,
    units::{Ellipse, LayoutSize},
};

/// Orientation of a straight line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineOrientation {
    /// Top-bottom line.
    Vertical,
    /// Left-right line.
    Horizontal,
}
impl From<LineOrientation> for w_api::LineOrientation {
    fn from(o: LineOrientation) -> Self {
        match o {
            LineOrientation::Vertical => w_api::LineOrientation::Vertical,
            LineOrientation::Horizontal => w_api::LineOrientation::Horizontal,
        }
    }
}

/// Represents a line style.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineStyle {
    /// A solid line.
    Solid,
    /// Two solid lines in parallel.
    Double,

    /// Dotted line.
    Dotted,
    /// Dashed line.
    Dashed,

    /// Faux shadow with carved appearance.
    Groove,
    /// Faux shadow with extruded appearance.
    Ridge,

    /// A wavy line, like an error underline.
    ///
    /// The wave magnitude is defined by the overall line thickness, the associated value
    /// here defines the thickness of the wavy line.
    Wavy(f32),

    /// Fully transparent line.
    Hidden,
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
impl From<BorderStyle> for w_api::BorderStyle {
    fn from(s: BorderStyle) -> Self {
        match s {
            BorderStyle::Solid => w_api::BorderStyle::Solid,
            BorderStyle::Double => w_api::BorderStyle::Double,
            BorderStyle::Dotted => w_api::BorderStyle::Dotted,
            BorderStyle::Dashed => w_api::BorderStyle::Dashed,
            BorderStyle::Hidden => w_api::BorderStyle::Hidden,
            BorderStyle::Groove => w_api::BorderStyle::Groove,
            BorderStyle::Ridge => w_api::BorderStyle::Ridge,
            BorderStyle::Inset => w_api::BorderStyle::Inset,
            BorderStyle::Outset => w_api::BorderStyle::Outset,
        }
    }
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
impl From<BorderSide> for w_api::BorderSide {
    fn from(s: BorderSide) -> Self {
        w_api::BorderSide {
            color: s.color.into(),
            style: s.style.into(),
        }
    }
}

/// Radius of each corner of a border defined from [`Ellipse`] values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BorderRadius {
    /// Top-left corner.
    pub top_left: Ellipse,
    /// Top-right corner.
    pub top_right: Ellipse,
    /// Bottom-left corner.
    pub bottom_left: Ellipse,
    /// Bottom-right corner.
    pub bottom_right: Ellipse,
}
impl BorderRadius {
    /// New every corner unique.
    pub fn new<TL: Into<Ellipse>, TR: Into<Ellipse>, BL: Into<Ellipse>, BR: Into<Ellipse>>(
        top_left: TL,
        top_right: TR,
        bottom_left: BL,
        bottom_right: BR,
    ) -> Self {
        BorderRadius {
            top_left: top_left.into(),
            top_right: top_right.into(),
            bottom_left: bottom_left.into(),
            bottom_right: bottom_right.into(),
        }
    }

    /// New all corners the same.
    pub fn uniform<E: Into<Ellipse>>(ellipse: E) -> Self {
        let e = ellipse.into();
        BorderRadius {
            top_left: e,
            top_right: e,
            bottom_left: e,
            bottom_right: e,
        }
    }

    /// No corner radius.
    #[inline]
    pub fn zero() -> Self {
        Self::uniform(Ellipse::zero())
    }

    /// Compute the radii in a layout context.
    #[inline]
    pub fn to_layout(self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutBorderRadius {
        LayoutBorderRadius {
            top_left: self.top_left.to_layout(available_size, ctx),
            top_right: self.top_right.to_layout(available_size, ctx),
            bottom_left: self.bottom_left.to_layout(available_size, ctx),
            bottom_right: self.bottom_right.to_layout(available_size, ctx),
        }
    }
}

/// Computed [`BorderRadius`].
pub type LayoutBorderRadius = w_api::BorderRadius;

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
            radius: BorderRadius::zero(),
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
            radius: BorderRadius::zero(),
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
            radius: BorderRadius::zero(),
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

    /// Compute the radii in a layout context and convert the sides to a normal webrender border.
    #[inline]
    pub fn to_layout(self, available_size: LayoutSize, ctx: &LayoutContext) -> LayoutBorderDetails {
        LayoutBorderDetails::Normal(w_api::NormalBorder {
            left: self.left.into(),
            right: self.right.into(),
            top: self.top.into(),
            bottom: self.bottom.into(),
            radius: self.radius.to_layout(available_size, ctx),
            do_aa: true,
        })
    }
}

/// Computed [`BorderDetails`].
///
/// You can use [`border_details_none`] to initialize this value.
pub type LayoutBorderDetails = w_api::BorderDetails;

/// Provides an initial value for [`LayoutBorderDetails`].
pub fn border_details_none() -> LayoutBorderDetails {
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
