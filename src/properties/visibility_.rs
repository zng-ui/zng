use crate::prelude::new_property::*;
use std::ops;

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

struct VisibilityNode<C: UiNode, V: VarLocal<Visibility>> {
    child: C,
    visibility: V,
}
impl<C: UiNode, V: VarLocal<Visibility>> VisibilityNode<C, V> {
    fn with_context(&mut self, vars: &Vars, f: impl FnOnce(&mut C)) {
        match *VisibilityVar::var().get(vars) {
            // parent collapsed => all descendants collapsed
            Visibility::Collapsed => f(&mut self.child),
            // parent hidden =>
            Visibility::Hidden => {
                // if we are collapsed
                if let Visibility::Collapsed = self.visibility.get(vars) {
                    // our branch is collapsed
                    let child = &mut self.child;
                    vars.with_context_bind(VisibilityVar, &self.visibility, || f(child));
                } else {
                    // otherwise same as parent
                    f(&mut self.child)
                }
            }
            // parent visible =>
            Visibility::Visible => {
                if let Visibility::Visible = self.visibility.get(vars) {
                    // and we are also visible, same as parent
                    f(&mut self.child)
                } else {
                    // or, our visibility is different
                    let child = &mut self.child;
                    vars.with_context_bind(VisibilityVar, &self.visibility, || f(child));
                }
            }
        }
    }
}
impl<C: UiNode, V: VarLocal<Visibility>> UiNode for VisibilityNode<C, V> {
    fn init(&mut self, ctx: &mut WidgetContext) {
        let vis = *self.visibility.init_local(ctx.vars);
        ctx.widget_state.set(VisibilityState, vis);

        self.with_context(ctx.vars, |c| c.init(ctx));
    }

    fn deinit(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.deinit(ctx));
    }

    fn update(&mut self, ctx: &mut WidgetContext) {
        if let Some(&vis) = self.visibility.update_local(ctx.vars) {
            ctx.widget_state.set(VisibilityState, vis);
            ctx.updates.layout();
        }
        self.with_context(ctx.vars, |c| c.update(ctx));
    }

    fn update_hp(&mut self, ctx: &mut WidgetContext) {
        self.with_context(ctx.vars, |c| c.update_hp(ctx));
    }

    fn measure(&mut self, available_size: LayoutSize, ctx: &mut LayoutContext) -> LayoutSize {
        match *self.visibility.get_local() {
            Visibility::Visible | Visibility::Hidden => self.child.measure(available_size, ctx),
            Visibility::Collapsed => LayoutSize::zero(),
        }
    }

    fn arrange(&mut self, final_size: LayoutSize, ctx: &mut LayoutContext) {
        if let Visibility::Visible = self.visibility.get_local() {
            self.child.arrange(final_size, ctx)
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if let Visibility::Visible = self.visibility.get_local() {
            self.child.render(frame);
        } else {
            frame
                .cancel_widget()
                .expect("visibility not set before `FrameBuilder::open_widget_display`");
        }
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if let Visibility::Visible = self.visibility.get_local() {
            self.child.render_update(update);
        } else {
            update.cancel_widget();
        }
    }
}

/// Sets the widget visibility.
#[property(context)]
pub fn visibility(child: impl UiNode, visibility: impl IntoVar<Visibility>) -> impl UiNode {
    VisibilityNode {
        child,
        visibility: visibility.into_local(),
    }
}

state_key! { struct VisibilityState: Visibility; }

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

/// Extension methods for filtering an [`UiList`] by [`Visibility`].
pub trait UiListVisibilityExt: UiList {
    /// Counts the widgets that are not collapsed.
    fn count_not_collapsed(&self) -> usize;

    /// Render widgets, calls `origin` only for widgets that are not collapsed.
    fn render_not_collapsed<O: FnMut(usize) -> LayoutPoint>(&self, origin: O, frame: &mut FrameBuilder);
}

impl<U: UiList> UiListVisibilityExt for U {
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

context_var! {
    /// Don't use this directly unless you read all the visibility related
    /// source code here and in core/window.rs
    #[doc(hidden)]
    pub struct VisibilityVar: Visibility = return &Visibility::Visible;
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
