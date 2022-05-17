use super::{euclid, Px, PxSize};

/// If the `max` size is the *fill* size, otherwise `min` is the *fill* size.
///
/// See [`LayoutConstrains`] for more details.
pub type FillVector = euclid::BoolVector2D;

/// Constrains on a pixel length.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PxConstrains {
    /// Maximum allowed length.
    pub max: Px,
    /// Minimum allowed length.
    pub min: Px,
    /// If `max` is the *fill* length, otherwise `min` is.
    pub fill: bool,
}
impl Default for PxConstrains {
    fn default() -> Self {
        Self {
            max: Px::MAX,
            min: Px(0),
            fill: false,
        }
    }
}
impl PxConstrains {
    /// No constrains, max is [`Px::MAX`], min is zero and fill is false, this the default value.
    pub fn none() -> Self {
        Self::default()
    }

    /// Fixed length constrains, both max and min are `px`, fill is false.
    pub fn fixed(px: Px) -> Self {
        Self {
            max: px,
            min: px,
            fill: false,
        }
    }

    /// Returns the length to fill.
    pub fn fill_length(&self) -> Px {
        if self.fill {
            self.max
        } else {
            self.min
        }
    }

    /// Clamp the `px` by min and max.
    pub fn clamp(&self, px: Px) -> Px {
        self.min.max(px).min(self.max)
    }

    /// Returns a constrain with `max`.
    pub fn with_max(mut self, max: Px) -> Self {
        self.max = max;
        self
    }

    /// Returns a constrain with `min`.
    pub fn with_min(mut self, min: Px) -> Self {
        self.min = min;
        self
    }

    /// Returns a constrain with fill config.
    pub fn with_fill(mut self, fill: bool) -> Self {
        self.fill = fill;
        self
    }

    /// Returns a constrains with `max` subtracted by `removed` and `min` adjusted to be less-or-equal to `max`.
    pub fn with_less(mut self, removed: Px) -> Self {
        self.max -= removed;
        self.min = self.min.min(self.max);
        self
    }

    /// Returns a constrains with `max` added by `added`
    pub fn with_more(mut self, added: Px) -> Self {
        self.max += added;
        self
    }
}

/// Constrains on a pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PxSizeConstrains {
    /// Maximum allowed size.
    pub max: PxSize,
    /// Minimum allowed size.
    pub min: PxSize,
    /// If `max` size is the *fill* size, otherwise `min` is.
    pub fill: FillVector,
}
impl Default for PxSizeConstrains {
    fn default() -> Self {
        PxSizeConstrains {
            max: PxSize::new(Px::MAX, Px::MAX),
            min: PxSize::zero(),
            fill: FillVector { x: false, y: false },
        }
    }
}
impl PxSizeConstrains {
    /// No constrains, max is [`Px::MAX`], min is zero and fill is false, this the default value.
    pub fn none() -> Self {
        Self::default()
    }

    /// Fixed size constrains, both max and min are `size`, fill is false.
    pub fn fixed(size: PxSize) -> Self {
        Self {
            max: size,
            min: size,
            fill: FillVector { x: false, y: false },
        }
    }

    /// Returns the size to fill all available space.
    pub fn fill_size(&self) -> PxSize {
        debug_assert!(self.max.width >= self.min.width);
        debug_assert!(self.max.height >= self.min.height);

        self.fill.select_size(self.max, self.min)
    }

    /// Returns the width that fills the X-axis.
    pub fn fill_width(&self) -> Px {
        if self.fill.x {
            self.max.width
        } else {
            self.min.width
        }
    }

    /// Returns the height that fills the Y-axis.
    pub fn fill_height(&self) -> Px {
        if self.fill.y {
            self.max.height
        } else {
            self.min.height
        }
    }

    /// Clamp the `size` by min and max.
    pub fn clamp(&self, size: PxSize) -> PxSize {
        self.min.max(size).min(self.max)
    }

    /// X-axis constrains.
    pub fn x_constrains(&self) -> PxConstrains {
        PxConstrains {
            max: self.max.width,
            min: self.min.width,
            fill: self.fill.x,
        }
    }

    /// Y-axis constrains.
    pub fn y_constrains(&self) -> PxConstrains {
        PxConstrains {
            max: self.max.height,
            min: self.min.height,
            fill: self.fill.y,
        }
    }

    /// Returns a constrain with `max` size and `min` adjusted to be less-or-equal to `max`.
    pub fn with_max(mut self, max: PxSize) -> Self {
        self.max = max;
        self.min = self.min.min(self.max);
        self
    }

    /// Returns a constrain with `max` size, `min` adjusted to be less-or-equal to `max` and fill set to both.
    pub fn with_max_fill(self, max: PxSize) -> Self {
        self.with_max(max).with_fill(true, true)
    }

    /// Returns a constrain with `min` size and `max` adjusted to be more-or-equal to `min`.
    pub fn with_min(mut self, min: PxSize) -> Self {
        self.min = min;
        self.max = self.max.max(self.min);
        self
    }

    /// Returns a constrain with `max.width` size and `min.width` adjusted to be less-or-equal to `max.width`.
    pub fn with_max_width(mut self, max_width: Px) -> Self {
        self.max.width = max_width;
        self.min.width = self.min.width.min(self.max.width);
        self
    }

    /// Returns a constrain with `max.width` size, `min.width` adjusted to be less-or-equal to `max.width` and `fill.x` set.
    pub fn with_width_fill(self, max_width: Px) -> Self {
        self.with_max_width(max_width).with_fill_x(true)
    }

    /// Returns a constrain with `max.height` size and `min.height` adjusted to be less-or-equal to `max.height`.
    pub fn with_max_height(mut self, max_height: Px) -> Self {
        self.max.height = max_height;
        self.min.height = self.min.height.min(self.max.height);
        self
    }

    /// Returns a constrain with `max.height` size, `min.height` adjusted to be less-or-equal to `max.height` and `fill.y` set.
    pub fn with_height_fill(self, max_height: Px) -> Self {
        self.with_max_height(max_height).with_fill_y(true)
    }

    /// Returns a constrain with `min.width` size and `max.width` adjusted to be more-or-equal to `min.width`.
    pub fn with_min_width(mut self, min_width: Px) -> Self {
        self.min.width = min_width;
        self.max.width = self.max.width.max(self.min.width);
        self
    }

    /// Returns a constrain with `max.height` size and `max.height` adjusted to be more-or-equal to `min.height`.
    pub fn with_min_height(mut self, min_height: Px) -> Self {
        self.min.height = min_height;
        self.max.height = self.max.height.max(self.min.height);
        self
    }

    /// Returns a constrain with fill config in both axis.
    pub fn with_fill(mut self, fill_x: bool, fill_y: bool) -> Self {
        self.fill = FillVector { x: fill_x, y: fill_y };
        self
    }

    /// Returns a constrain with `fill.x` config.
    pub fn with_fill_x(mut self, fill_x: bool) -> Self {
        self.fill.x = fill_x;
        self
    }

    /// Returns a constrain with `fill.y` config.
    pub fn with_fill_y(mut self, fill_y: bool) -> Self {
        self.fill.y = fill_y;
        self
    }

    /* Note, Px ops are saturating */

    /// Returns a constrains with `max` subtracted by `removed` and `min` adjusted to be less-or-equal to `max`.
    pub fn with_less_size(mut self, removed: PxSize) -> Self {
        self.max -= removed;
        self.min = self.min.min(self.max);
        self
    }

    /// Returns a constrains with `max.width` subtracted by `removed` and `min.width` adjusted to be less-or-equal to `max.width`.
    pub fn with_less_width(mut self, removed: Px) -> Self {
        self.max.width -= removed;
        self.min.width = self.min.width.min(self.max.width);
        self
    }

    /// Returns a constrains with `max.height` subtracted by `removed` and `min.height` adjusted to be less-or-equal to `max.height`.
    pub fn with_less_height(mut self, removed: Px) -> Self {
        self.max.height -= removed;
        self.min.height = self.min.height.min(self.max.height);
        self
    }

    /// Returns a constrains with `max` added by `added`.
    pub fn with_more_size(mut self, added: PxSize) -> Self {
        self.max -= added;
        self
    }

    /// Returns a constrains with `max.width` added by `added`.
    pub fn with_more_width(mut self, added: Px) -> Self {
        self.max.width -= added;
        self
    }

    /// Returns a constrains with `max.height` added by `added`.
    pub fn with_more_height(mut self, added: Px) -> Self {
        self.max.height += added;
        self
    }

    /// Returns a constrains with `max.width` set to MAX, `min.width` set to zero and `fill.x` set to false.
    pub fn with_unbounded_x(mut self) -> Self {
        self.max.width = Px::MAX;
        self.min.width = Px(0);
        self.fill.x = false;
        self
    }

    /// Returns a constrains with `max.height` set to MAX, `min.height` set to zero and `fill.y` set to false.
    pub fn with_unbounded_y(mut self) -> Self {
        self.max.height = Px::MAX;
        self.min.height = Px(0);
        self.fill.y = false;
        self
    }
}
