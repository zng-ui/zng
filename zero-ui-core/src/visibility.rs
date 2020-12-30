use std::ops;

use crate::context::{state_key, LazyStateMap};
use crate::render::FrameBuilder;
use crate::units::LayoutPoint;
use crate::var::{context_var, Vars};
use crate::{Widget, WidgetList};

/// Widget visibility.
///
/// The visibility value affects the widget and its descendants.
///
/// # Inheritance
///
/// In a UI tree the visibility of widgets combine with that of their parents.
///
/// * If the parent is collapsed all descendants are collapsed.
///
/// * If the parent is hidden some descendants can still be collapsed and affect the layout.
///
/// * If the parent is visible the descendants can have the other visibility modes.
///
/// This combination of visibility is implemented as a *bit OR* (`|`) operation.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum Visibility {
    /// The widget is visible, this is default.
    Visible,
    /// The widget is not visible, but still affects layout.
    ///
    /// Hidden widgets measure and reserve space in their parent but are not present
    /// in the rendered frames.
    Hidden,
    /// The widget is not visible and does not affect layout.
    ///
    /// Collapsed widgets always measure to zero and are not included in the rendered frames.
    ///
    /// Layout widgets also consider this value, [`uniform_grid!`](mod@crate::widgets::layouts::uniform_grid) does not
    /// count collapsed widgets when reserving cells.
    Collapsed,
}
impl Default for Visibility {
    /// [` Visibility::Visible`]
    fn default() -> Self {
        Visibility::Visible
    }
}
impl ops::BitOr for Visibility {
    type Output = Self;

    /// `Collapsed` | `Hidden` | `Visible` short circuit from left to right.
    fn bitor(self, rhs: Self) -> Self::Output {
        use Visibility::*;
        match (self, rhs) {
            (Collapsed, _) | (_, Collapsed) => Collapsed,
            (Hidden, _) | (_, Hidden) => Hidden,
            _ => Visible,
        }
    }
}
impl ops::BitOrAssign for Visibility {
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}
impl_from_and_into_var! {
    /// * `true` -> `Visible`
    /// * `false` -> `Collapsed`
    fn from(visible: bool) -> Visibility {
        if visible { Visibility::Visible } else { Visibility::Collapsed }
    }
}

state_key! { struct VisibilityState: Visibility; }

context_var! {
    /// Don't use this directly unless you read all the visibility related
    /// source code here and in core/window.rs
    #[doc(hidden)]
    pub struct VisibilityVar: Visibility = return &Visibility::Visible;
}

/// Extension method for accessing the [`Visibility`] of widgets.
pub trait WidgetVisibilityExt {
    /// Gets the widget visibility.
    ///
    /// This gets only the visibility configured in the widget, if a parent widget
    /// is not visible that does not show here. Use [`VisibilityContext`] to get the inherited
    /// visibility from inside a widget.
    fn visibility(&self) -> Visibility;
}
impl WidgetVisibilityExt for LazyStateMap {
    fn visibility(&self) -> Visibility {
        self.get(VisibilityState).copied().unwrap_or_default()
    }
}
impl<W: Widget> WidgetVisibilityExt for W {
    fn visibility(&self) -> Visibility {
        self.state().visibility()
    }
}

/// Extension methods for filtering an [`WidgetList`] by [`Visibility`].
pub trait WidgetListVisibilityExt: WidgetList {
    /// Counts the widgets that are not collapsed.
    fn count_not_collapsed(&self) -> usize;

    /// Render widgets, calls `origin` only for widgets that are not collapsed.
    fn render_not_collapsed<O: FnMut(usize) -> LayoutPoint>(&self, origin: O, frame: &mut FrameBuilder);
}

impl<U: WidgetList> WidgetListVisibilityExt for U {
    fn count_not_collapsed(&self) -> usize {
        self.count(|_, s| s.visibility() != Visibility::Collapsed)
    }

    fn render_not_collapsed<O: FnMut(usize) -> LayoutPoint>(&self, mut origin: O, frame: &mut FrameBuilder) {
        self.render_filtered(
            |i, s| {
                if s.visibility() != Visibility::Collapsed {
                    Some(origin(i))
                } else {
                    None
                }
            },
            frame,
        )
    }
}

/// Contextual [`Visibility`] accessor.
pub struct VisibilityContext;
impl VisibilityContext {
    /// Gets the visibility state in the current `vars` context.
    #[inline]
    pub fn get(vars: &Vars) -> Visibility {
        *VisibilityVar::var().get(vars)
    }
}
