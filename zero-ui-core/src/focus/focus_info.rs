use crate::{
    context::*,
    units::*,
    var::impl_from_and_into_var,
    widget_info::{Interactivity, TreeFilter, Visibility, WidgetInfo, WidgetInfoTree, WidgetPath},
    window::FocusIndicator,
    WidgetId,
};
use std::fmt;

use super::iter::IterFocusableExt;

state_key! {
    /// Reference to the [`FocusInfoBuilder`] in the widget state.
    pub struct FocusInfoKey: FocusInfoBuilder;
}

/// Widget tab navigation position within a focus scope.
///
/// The index is zero based, zero first.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct TabIndex(pub u32);
impl TabIndex {
    /// Widget is skipped during tab navigation.
    ///
    /// The integer value is `u32::MAX`.
    pub const SKIP: TabIndex = TabIndex(u32::MAX);

    /// Default focusable widget index.
    ///
    /// Tab navigation uses the widget position in the widget tree when multiple widgets have the same index
    /// so if no widget index is explicitly set they get auto-sorted by their position.
    ///
    /// The integer value is `u32::MAX / 2`.
    pub const AUTO: TabIndex = TabIndex(u32::MAX / 2);

    /// If is [`SKIP`](TabIndex::SKIP).
    pub fn is_skip(self) -> bool {
        self == Self::SKIP
    }

    /// If is [`AUTO`](TabIndex::AUTO).
    pub fn is_auto(self) -> bool {
        self == Self::AUTO
    }

    /// If is a custom index placed [before auto](Self::before_auto).
    pub fn is_before_auto(self) -> bool {
        self.0 < Self::AUTO.0
    }

    /// If is a custom index placed [after auto](Self::after_auto).
    pub fn is_after_auto(self) -> bool {
        self.0 > Self::AUTO.0
    }

    /// Create a new tab index that is guaranteed to not be [`SKIP`](Self::SKIP).
    ///
    /// Returns `SKIP - 1` if `index` is `SKIP`.
    pub fn not_skip(index: u32) -> Self {
        TabIndex(if index == Self::SKIP.0 { Self::SKIP.0 - 1 } else { index })
    }

    /// Create a new tab index that is guaranteed to be before [`AUTO`](Self::AUTO).
    ///
    /// Returns `AUTO - 1` if `index` is equal to or greater then `AUTO`.
    pub fn before_auto(index: u32) -> Self {
        TabIndex(if index >= Self::AUTO.0 { Self::AUTO.0 - 1 } else { index })
    }

    /// Create a new tab index that is guaranteed to be after [`AUTO`](Self::AUTO) and not [`SKIP`](Self::SKIP).
    ///
    /// The `index` argument is zero based here.
    ///
    /// Returns `not_skip(AUTO + 1 + index)`.
    pub fn after_auto(index: u32) -> Self {
        Self::not_skip((Self::AUTO.0 + 1).saturating_add(index))
    }
}
impl fmt::Debug for TabIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            if self.is_auto() {
                write!(f, "TabIndex::AUTO")
            } else if self.is_skip() {
                write!(f, "TabIndex::SKIP")
            } else if self.is_after_auto() {
                write!(f, "TabIndex::after_auto({})", self.0 - Self::AUTO.0 - 1)
            } else {
                write!(f, "TabIndex({})", self.0)
            }
        } else {
            //
            if self.is_auto() {
                write!(f, "AUTO")
            } else if self.is_skip() {
                write!(f, "SKIP")
            } else if self.is_after_auto() {
                write!(f, "after_auto({})", self.0 - Self::AUTO.0 - 1)
            } else {
                write!(f, "{}", self.0)
            }
        }
    }
}
impl Default for TabIndex {
    /// `AUTO`
    fn default() -> Self {
        TabIndex::AUTO
    }
}
impl_from_and_into_var! {
    /// Calls [`TabIndex::not_skip`].
    fn from(index: u32) -> TabIndex {
        TabIndex::not_skip(index)
    }
}

/// Tab navigation configuration of a focus scope.
///
/// See the [module level](crate::focus#tab-navigation) for an overview of tab navigation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabNav {
    /// Tab can move into the scope, but does not move the focus inside the scope.
    None,
    /// Tab moves the focus through the scope continuing out after the last item.
    Continue,
    /// Tab is contained in the scope, does not move after the last item.
    Contained,
    /// Tab is contained in the scope, after the last item moves to the first item in the scope.
    Cycle,
    /// Tab moves into the scope once but then moves out of the scope.
    Once,
}
impl fmt::Debug for TabNav {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "TabNav::")?;
        }
        match self {
            TabNav::None => write!(f, "None"),
            TabNav::Continue => write!(f, "Continue"),
            TabNav::Contained => write!(f, "Contained"),
            TabNav::Cycle => write!(f, "Cycle"),
            TabNav::Once => write!(f, "Once"),
        }
    }
}

/// Directional navigation configuration of a focus scope.
///
/// See the [module level](crate::focus#directional-navigation) for an overview of directional navigation.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionalNav {
    /// Arrows can move into the scope, but does not move the focus inside the scope.
    None,
    /// Arrows move the focus through the scope continuing out of the edges.
    Continue,
    /// Arrows move the focus inside the scope only, stops at the edges.
    Contained,
    /// Arrows move the focus inside the scope only, cycles back to oppose edges.
    Cycle,
}
impl fmt::Debug for DirectionalNav {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "DirectionalNav::")?;
        }
        match self {
            DirectionalNav::None => write!(f, "None"),
            DirectionalNav::Continue => write!(f, "Continue"),
            DirectionalNav::Contained => write!(f, "Contained"),
            DirectionalNav::Cycle => write!(f, "Cycle"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Focus change request.
///
/// See [`Focus`] for details.
///
/// [`Focus`]: crate::focus::Focus::focus
pub struct FocusRequest {
    /// Where to move the focus.
    pub target: FocusTarget,
    /// If the widget should visually indicate that it has keyboard focus.
    pub highlight: bool,

    /// If the window should be focused even if another app has focus. By default the window
    /// is only focused if the app has keyboard focus in any of the open windows, if this is enabled
    /// a [`Windows::focus`] request is always made, potentially stealing keyboard focus from another app
    /// and disrupting the user.
    ///
    /// [`Windows::focus`]: crate::window::Windows::focus
    pub force_window_focus: bool,

    /// Focus indicator to set on the target window if the app does not have keyboard focus and
    /// `force_window_focus` is disabled.
    ///
    /// The [`focus_indicator`] of the window is set and the request is processed after the window receives focus,
    /// or it is canceled if another focus request is made.
    ///
    /// [`focus_indicator`]: crate::window::WindowVars::focus_indicator
    pub window_indicator: Option<FocusIndicator>,
}

impl FocusRequest {
    #[allow(missing_docs)]
    pub fn new(target: FocusTarget, highlight: bool) -> Self {
        Self {
            target,
            highlight,
            force_window_focus: false,
            window_indicator: None,
        }
    }

    /// New [`FocusTarget::Direct`] request.
    pub fn direct(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::Direct(widget_id), highlight)
    }
    /// New [`FocusTarget::DirectOrExit`] request.
    pub fn direct_or_exit(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::DirectOrExit(widget_id), highlight)
    }
    /// New [`FocusTarget::DirectOrEnder`] request.
    pub fn direct_or_enter(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::DirectOrEnder(widget_id), highlight)
    }
    /// New [`FocusTarget::DirectOrRelated`] request.
    pub fn direct_or_related(widget_id: WidgetId, highlight: bool) -> Self {
        Self::new(FocusTarget::DirectOrRelated(widget_id), highlight)
    }
    /// New [`FocusTarget::Enter`] request.
    pub fn enter(highlight: bool) -> Self {
        Self::new(FocusTarget::Enter, highlight)
    }
    /// New [`FocusTarget::Exit`] request.
    pub fn exit(highlight: bool) -> Self {
        Self::new(FocusTarget::Exit, highlight)
    }
    /// New [`FocusTarget::Next`] request.
    pub fn next(highlight: bool) -> Self {
        Self::new(FocusTarget::Next, highlight)
    }
    /// New [`FocusTarget::Prev`] request.
    pub fn prev(highlight: bool) -> Self {
        Self::new(FocusTarget::Prev, highlight)
    }
    /// New [`FocusTarget::Up`] request.
    pub fn up(highlight: bool) -> Self {
        Self::new(FocusTarget::Up, highlight)
    }
    /// New [`FocusTarget::Right`] request.
    pub fn right(highlight: bool) -> Self {
        Self::new(FocusTarget::Right, highlight)
    }
    /// New [`FocusTarget::Down`] request.
    pub fn down(highlight: bool) -> Self {
        Self::new(FocusTarget::Down, highlight)
    }
    /// New [`FocusTarget::Left`] request.
    pub fn left(highlight: bool) -> Self {
        Self::new(FocusTarget::Left, highlight)
    }
    /// New [`FocusTarget::Alt`] request.
    pub fn alt(highlight: bool) -> Self {
        Self::new(FocusTarget::Alt, highlight)
    }

    /// Sets [`FocusRequest::force_window_focus`] to `true`.
    pub fn with_force_window_focus(mut self) -> Self {
        self.force_window_focus = true;
        self
    }

    /// Sets the [`FocusRequest::window_indicator`].
    pub fn with_indicator(mut self, indicator: FocusIndicator) -> Self {
        self.window_indicator = Some(indicator);
        self
    }
}

/// Focus request target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    /// Move focus to widget.
    Direct(WidgetId),
    /// Move focus to the widget if it is focusable or to the first focusable ancestor.
    DirectOrExit(WidgetId),
    /// Move focus to the widget if it is focusable or to first focusable descendant.
    DirectOrEnder(WidgetId),
    /// Move focus to the widget if it is focusable, or to the first focusable descendant or
    /// to the first focusable ancestor.
    DirectOrRelated(WidgetId),

    /// Move focus to the first focusable descendant of the current focus, or to first in screen.
    Enter,
    /// Move focus to the first focusable ancestor of the current focus, or to first in screen, or the return focus from ALT scopes.
    Exit,

    /// Move focus to next from current in screen, or to first in screen.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,

    /// Move focus above current.
    Up,
    /// Move focus to the right of current.
    Right,
    /// Move focus bellow current.
    Down,
    /// Move focus to the left of current.
    Left,

    /// Move focus to the current widget ALT scope or out of it.
    Alt,
}

bitflags! {
    /// Represents the [`FocusTarget`] actions that move focus from the current focused widget.
    pub struct FocusNavAction: u16 {
        /// [`FocusTarget::Enter`]
        const ENTER =      0b0000_0000_0001;
        /// [`FocusTarget::Exit`]
        const EXIT =       0b0000_0000_0010;

        /// [`FocusTarget::Next`]
        const NEXT =       0b0000_0000_0100;
        /// [`FocusTarget::Prev`]
        const PREV =       0b0000_0000_1000;

        /// [`FocusTarget::Up`]
        const UP =         0b0000_0001_0000;
        /// [`FocusTarget::Right`]
        const RIGHT =      0b0000_0010_0000;
        /// [`FocusTarget::Down`]
        const DOWN =       0b0000_0100_0000;
        /// [`FocusTarget::Left`]
        const LEFT =       0b0000_1000_0000;

        /// [`FocusTarget::Alt`]
        const ALT =        0b0001_0000_0000;

        /// Up, right, down, left.
        const DIRECTIONAL = FocusNavAction::UP.bits | FocusNavAction::RIGHT.bits | FocusNavAction::DOWN.bits | FocusNavAction::LEFT.bits;
    }
}

/// A [`WidgetInfoTree`] wrapper for querying focus info out of the widget tree.
#[derive(Copy, Clone, Debug)]
pub struct FocusInfoTree<'a> {
    /// Full widget info.
    pub tree: &'a WidgetInfoTree,
    min_interactivity: Interactivity,
}
impl<'a> FocusInfoTree<'a> {
    /// Wrap a `widget_info` reference to enable focus info querying.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details on the second parameter.
    ///
    /// [`Focus::focus_disabled_widgets`]: crate::focus::Focus::focus_disabled_widgets
    pub fn new(tree: &'a WidgetInfoTree, focus_disabled_widgets: bool) -> Self {
        FocusInfoTree {
            tree,
            min_interactivity: if focus_disabled_widgets {
                Interactivity::DISABLED
            } else {
                Interactivity::ENABLED
            },
        }
    }

    /// If [`DISABLED`] widgets are focusable in this tree.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details.
    ///
    /// [`DISABLED`]: Interactivity::DISABLED
    /// [`Focus::focus_disabled_widgets`]: crate::focus::Focus::focus_disabled_widgets
    pub fn focus_disabled_widgets(&self) -> bool {
        self.min_interactivity.is_disabled()
    }

    /// Reference to the root widget in the tree.
    ///
    /// The root is usually a focusable focus scope but it may not be. This
    /// is the only method that returns a [`WidgetFocusInfo`] that may not be focusable.
    pub fn root(&self) -> WidgetFocusInfo {
        WidgetFocusInfo {
            info: self.tree.root(),
            min_interactivity: self.min_interactivity,
        }
    }

    /// Reference the focusable widget closest to the window root.
    ///
    /// When the window root is not focusable, but a descendant widget is, this method returns
    /// the focusable closest to the root counting previous siblings then parents.
    pub fn focusable_root(&self) -> Option<WidgetFocusInfo> {
        let root = self.root();
        if root.is_focusable() {
            return Some(root);
        }

        let mut candidate = None;
        let mut candidate_weight = usize::MAX;

        for w in root.descendants().filter(|_| TreeFilter::SkipDescendants) {
            let weight = w.info.prev_siblings().count() + w.info.ancestors().count();
            if weight < candidate_weight {
                candidate = Some(w);
                candidate_weight = weight;
            }
        }

        candidate
    }

    /// Reference to the widget in the tree, if it is present and is focusable.
    pub fn find(&self, widget_id: impl Into<WidgetId>) -> Option<WidgetFocusInfo> {
        self.tree
            .find(widget_id)
            .and_then(|i| i.as_focusable(self.focus_disabled_widgets()))
    }

    /// Reference to the widget in the tree, if it is present and is focusable.
    ///
    /// Faster then [`find`](Self::find) if the widget path was generated by the same tree.
    pub fn get(&self, path: &WidgetPath) -> Option<WidgetFocusInfo> {
        self.tree.get(path).and_then(|i| i.as_focusable(self.focus_disabled_widgets()))
    }

    /// Reference to the first focusable widget or parent in the tree.
    pub fn get_or_parent(&self, path: &WidgetPath) -> Option<WidgetFocusInfo> {
        self.get(path)
            .or_else(|| path.ancestors().iter().rev().find_map(|&id| self.find(id)))
    }

    /// If the tree info contains the widget and it is focusable.
    pub fn contains(&self, widget_id: impl Into<WidgetId>) -> bool {
        self.find(widget_id).is_some()
    }
}

/// [`WidgetInfo`] extensions that build a [`WidgetFocusInfo`].
pub trait WidgetInfoFocusExt<'a> {
    /// Wraps the [`WidgetInfo`] in a [`WidgetFocusInfo`] even if it is not focusable.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details on the second parameter.
    ///
    /// [`Focus::focus_disabled_widgets`]: crate::focus::Focus::focus_disabled_widgets
    #[allow(clippy::wrong_self_convention)] // WidgetFocusInfo is a reference wrapper.
    fn as_focus_info(self, focus_disabled_widgets: bool) -> WidgetFocusInfo<'a>;

    /// Returns a wrapped [`WidgetFocusInfo`] if the [`WidgetInfo`] is focusable.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details on the second parameter.
    ///
    /// [`Focus::focus_disabled_widgets`]: crate::focus::Focus::focus_disabled_widgets
    #[allow(clippy::wrong_self_convention)] // WidgetFocusInfo is a reference wrapper.
    fn as_focusable(self, focus_disabled_widgets: bool) -> Option<WidgetFocusInfo<'a>>;
}
impl<'a> WidgetInfoFocusExt<'a> for WidgetInfo<'a> {
    fn as_focus_info(self, focus_disabled_widgets: bool) -> WidgetFocusInfo<'a> {
        WidgetFocusInfo::new(self, focus_disabled_widgets)
    }
    fn as_focusable(self, focus_disabled_widgets: bool) -> Option<WidgetFocusInfo<'a>> {
        let r = self.as_focus_info(focus_disabled_widgets);
        if r.is_focusable() {
            Some(r)
        } else {
            None
        }
    }
}

/// [`WidgetInfo`] wrapper that adds focus information for each widget.
#[derive(Clone, Copy, Eq, PartialEq, Hash, Debug)]
pub struct WidgetFocusInfo<'a> {
    /// Full widget info.
    pub info: WidgetInfo<'a>,
    min_interactivity: Interactivity,
}
macro_rules! DirectionFn {
    (impl) => { impl Fn(PxPoint, PxPoint) -> (Px, Px, Px, Px) };
    (up) => { |from_pt, cand_c| (cand_c.y, from_pt.y, cand_c.x, from_pt.x) };
    (down) => { |from_pt, cand_c| (from_pt.y, cand_c.y, cand_c.x, from_pt.x) };
    (left) => { |from_pt, cand_c| (cand_c.x, from_pt.x, cand_c.y, from_pt.y) };
    (right) => { |from_pt, cand_c| (from_pt.x, cand_c.x, cand_c.y, from_pt.y) };
}
impl<'a> WidgetFocusInfo<'a> {
    /// Wrap a `widget_info` reference to enable focus info querying.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details on the second parameter.
    ///
    /// [`Focus::focus_disabled_widgets`]:  crate::focus::Focus::focus_disabled_widgets
    pub fn new(widget_info: WidgetInfo<'a>, focus_disabled_widgets: bool) -> Self {
        WidgetFocusInfo {
            info: widget_info,
            min_interactivity: if focus_disabled_widgets {
                Interactivity::DISABLED
            } else {
                Interactivity::ENABLED
            },
        }
    }

    /// If [`DISABLED`] widgets are focusable in this tree.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details.
    ///
    /// [`DISABLED`]: crate::widget_info::Interactivity::DISABLED
    /// [`Focus::focus_disabled_widgets`]: crate::focus::Focus::focus_disabled_widgets
    pub fn focus_disabled_widgets(&self) -> bool {
        self.min_interactivity.is_disabled()
    }

    /// Root focusable.
    pub fn root(self) -> Self {
        self.ancestors().last().unwrap_or(self)
    }

    /// If the widget is focusable.
    ///
    /// ## Note
    ///
    /// This is probably `true`, the only way to get a [`WidgetFocusInfo`] for a non-focusable widget is by
    /// calling [`as_focus_info`](WidgetInfoFocusExt::as_focus_info) or explicitly constructing one.
    ///
    /// Focus scopes are also focusable.
    pub fn is_focusable(self) -> bool {
        self.focus_info().is_focusable()
    }

    /// Is focus scope.
    pub fn is_scope(self) -> bool {
        self.focus_info().is_scope()
    }

    /// Is ALT focus scope.
    pub fn is_alt_scope(self) -> bool {
        self.focus_info().is_alt_scope()
    }

    /// Widget focus metadata.
    pub fn focus_info(self) -> FocusInfo {
        if self.info.visibility() != Visibility::Visible || self.info.interactivity() > self.min_interactivity {
            FocusInfo::NotFocusable
        } else if let Some(builder) = self.info.meta().get(FocusInfoKey) {
            builder.build()
        } else {
            FocusInfo::NotFocusable
        }
    }

    /// Iterator over focusable parent -> grandparent -> .. -> root.
    pub fn ancestors(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().focusable(self.focus_disabled_widgets())
    }

    /// Iterator over focus scopes parent -> grandparent -> .. -> root.
    pub fn scopes(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().filter_map(move |i| {
            let i = i.as_focus_info(self.focus_disabled_widgets());
            if i.is_scope() {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Reference to the focusable parent that contains this widget.
    pub fn parent(self) -> Option<WidgetFocusInfo<'a>> {
        self.ancestors().next()
    }

    /// Reference the focus scope parent that contains the widget.
    pub fn scope(self) -> Option<WidgetFocusInfo<'a>> {
        self.scopes().next()
    }

    /// Reference the ALT focus scope *closest* with the current widget.
    ///
    /// # Closest Alt Scope
    ///
    /// - If `self` is already an ALT scope or is in one, moves to a sibling ALT scope, nested ALT scopes are ignored.
    /// - If `self` is a normal scope, moves to the first descendant ALT scope, otherwise..
    /// - Recursively searches for an ALT scope sibling up the scope tree.
    pub fn alt_scope(self) -> Option<WidgetFocusInfo<'a>> {
        let _span = tracing::trace_span!("alt_scope").entered();
        if self.in_alt_scope() {
            // We do not allow nested alt scopes, search for sibling focus scope.
            return self.scopes().find(|s| !s.is_alt_scope()).and_then(|s| s.alt_scope_query(s));
        } else if self.is_scope() {
            // if we are a normal scope, search for an inner ALT scope first.
            let r = self.descendants().find(|w| w.focus_info().is_alt_scope().into());
            if r.is_some() {
                return r;
            }
        }

        // search for a sibling alt scope and up the scopes tree.
        self.alt_scope_query(self)
    }
    fn alt_scope_query(self, skip: WidgetFocusInfo<'a>) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            // search for an ALT scope in our previous scope siblings.
            scope
                .descendants()
                .filter(|w| if w == skip { TreeFilter::SkipAll } else { TreeFilter::Include })
                .find(|w| w.focus_info().is_alt_scope())
                // if found no sibling ALT scope, do the same search for our scope.
                .or_else(|| scope.alt_scope_query(skip))
        } else {
            // we reached root, no ALT found.
            None
        }
    }

    /// Widget is in a ALT scope or is an ALT scope.
    pub fn in_alt_scope(self) -> bool {
        self.is_alt_scope() || self.scopes().any(|s| s.is_alt_scope())
    }

    /// Widget the focus needs to move to when `self` gets focused.
    ///
    /// # Input
    ///
    /// * `last_focused`: A function that returns the last focused widget within a focus scope identified by `WidgetId`.
    /// * `reverse`: If the focus is *reversing* into `self`.
    ///
    /// # Returns
    ///
    /// Returns the different widget the focus must move to after focusing in `self` that is a focus scope.
    ///
    /// If `self` is not a [`FocusScope`](FocusInfo::FocusScope) always returns `None`.
    pub fn on_focus_scope_move<'p>(
        self,
        last_focused: impl FnOnce(WidgetId) -> Option<&'p WidgetPath>,
        reverse: bool,
    ) -> Option<WidgetFocusInfo<'a>> {
        match self.focus_info() {
            FocusInfo::FocusScope { on_focus, .. } => match on_focus {
                FocusScopeOnFocus::FirstDescendant => {
                    if reverse {
                        self.last_tab_descendant()
                    } else {
                        self.first_tab_descendant()
                    }
                }
                FocusScopeOnFocus::LastFocused => last_focused(self.info.widget_id())
                    .and_then(|path| self.info.tree().get(path))
                    .and_then(|w| w.as_focusable(self.focus_disabled_widgets()))
                    .and_then(|f| {
                        if f.ancestors().any(|a| a == self) {
                            Some(f) // valid last focused
                        } else {
                            None
                        }
                    })
                    .or_else(|| {
                        if reverse {
                            self.last_tab_descendant()
                        } else {
                            self.first_tab_descendant()
                        }
                    }), // fallback
                FocusScopeOnFocus::Widget => None,
            },
            FocusInfo::NotFocusable | FocusInfo::Focusable { .. } => None,
        }
    }

    /// Iterator over the focusable widgets contained by this widget.
    pub fn descendants(self) -> super::iter::FocusableDescendants<'a> {
        super::iter::FocusableDescendants::new(self.info.descendants(), self.focus_disabled_widgets())
    }

    /// Iterator over self and the focusable widgets contained by it.
    pub fn self_and_descendants(self) -> super::iter::FocusableDescendants<'a> {
        super::iter::FocusableDescendants::new(self.info.self_and_descendants(), self.focus_disabled_widgets())
    }

    /// If the focusable has any focusable descendant that is not [`TabIndex::SKIP`]
    pub fn has_tab_descendant(self) -> bool {
        self.descendants().find(Self::filter_tab_skip).is_some()
    }

    /// First descendant considering TAB index.
    pub fn first_tab_descendant(self) -> Option<WidgetFocusInfo<'a>> {
        let mut best = (TabIndex::SKIP, self);

        for d in self.descendants().filter(Self::filter_tab_skip) {
            let idx = d.focus_info().tab_index();

            if idx < best.0 {
                best = (idx, d);
            }
        }

        if best.0.is_skip() {
            None
        } else {
            Some(best.1)
        }
    }

    /// Last descendant considering TAB index.
    pub fn last_tab_descendant(self) -> Option<WidgetFocusInfo<'a>> {
        let mut best = (-1i64, self);

        for d in self.descendants().filter(Self::filter_tab_skip).rev() {
            let idx = d.focus_info().tab_index().0 as i64;

            if idx > best.0 {
                best = (idx, d);
            }
        }

        if best.0 < 0 {
            None
        } else {
            Some(best.1)
        }
    }

    /// Iterator over all focusable widgets in the same scope after this widget.
    pub fn next_focusables(self) -> super::iter::FocusableDescendants<'a> {
        if let Some(scope) = self.scope() {
            super::iter::FocusableDescendants::new(self.info.next_siblings_in(scope.info), self.focus_disabled_widgets())
        } else {
            super::iter::FocusableDescendants::new(self.info.next_siblings_in(self.info), self.focus_disabled_widgets())
        }
    }

    /// Next focusable in the same scope after this widget.
    pub fn next_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_focusables().next()
    }

    fn filter_tab_skip(w: WidgetFocusInfo<'a>) -> TreeFilter {
        if w.focus_info().tab_index().is_skip() {
            TreeFilter::SkipAll
        } else {
            TreeFilter::Include
        }
    }

    /// Next focusable in the same scope after this widget respecting the TAB index.
    ///
    /// If `self` is set to [`TabIndex::SKIP`] returns the next non-skip focusable in the same scope after this widget.
    ///
    /// If `skip_self` is `true`, does not include widgets inside `self`.
    pub fn next_tab_focusable(self, skip_self: bool) -> Option<WidgetFocusInfo<'a>> {
        let self_index = self.focus_info().tab_index();

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes to next in widget tree.
            return self.next_focusables().find(Self::filter_tab_skip);
        }

        let mut best = (TabIndex::SKIP, self);

        if !skip_self {
            for d in self.descendants().filter(Self::filter_tab_skip) {
                let idx = d.focus_info().tab_index();

                if idx == self_index {
                    return Some(d);
                } else if idx < best.0 && idx > self_index {
                    best = (idx, d);
                }
            }
        }

        for s in self.next_focusables().filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index();

            if idx == self_index {
                return Some(s);
            } else if idx < best.0 && idx > self_index {
                best = (idx, s);
            }
        }

        for s in self.prev_focusables().filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index();

            if idx <= best.0 && idx > self_index {
                best = (idx, s);
            }
        }

        if best.0.is_skip() {
            None
        } else {
            Some(best.1)
        }

        // Note: if you change this method you also need to change `enabled_tab_nav_in_scope`
    }

    /// Iterator over all focusable widgets in the same scope before this widget in reverse.
    pub fn prev_focusables(self) -> super::iter::FocusableDescendants<'a> {
        if let Some(scope) = self.scope() {
            super::iter::FocusableDescendants::new(self.info.prev_siblings_in(scope.info), self.focus_disabled_widgets())
        } else {
            super::iter::FocusableDescendants::new(self.info.prev_siblings_in(self.info), self.focus_disabled_widgets())
        }
    }

    /// Previous focusable in the same scope before this widget.
    pub fn prev_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        self.prev_focusables().next()
    }

    /// Previous focusable in the same scope after this widget respecting the TAB index.
    ///
    /// If `self` is set to [`TabIndex::SKIP`] returns the previous non-skip focusable in the same scope before this widget.
    ///
    /// If `skip_self` is `true`, does not include widgets inside `self`.
    pub fn prev_tab_focusable(self, skip_self: bool) -> Option<WidgetFocusInfo<'a>> {
        let self_index = self.focus_info().tab_index();

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes to prev in widget tree.
            return self.prev_focusables().find(Self::filter_tab_skip);
        }

        let self_index = self_index.0 as i64;
        let mut best = (-1i64, self);

        if !skip_self {
            for d in self.descendants().filter(Self::filter_tab_skip).rev() {
                let idx = d.focus_info().tab_index().0 as i64;

                if idx == self_index {
                    return Some(d);
                } else if idx > best.0 && idx < self_index {
                    best = (idx, d);
                }
            }
        }

        for s in self.prev_focusables().filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index().0 as i64;

            if idx == self_index {
                return Some(s);
            } else if idx > best.0 && idx < self_index {
                best = (idx, s);
            }
        }

        for s in self.next_focusables().filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index().0 as i64;

            if idx >= best.0 && idx < self_index {
                best = (idx, s);
            }
        }

        if best.0 < 0 {
            None
        } else {
            Some(best.1)
        }

        // Note: if you change this method you also need to change `enabled_tab_nav_in_scope`
    }

    /// Widget to focus when pressing TAB from this widget.
    ///
    /// Set `skip_self` to not enter `self`, that is, the focus goes to the next sibling or next sibling descendant.
    ///
    /// Returns `None` if the focus does not move to another widget.
    pub fn next_tab(self, skip_self: bool) -> Option<WidgetFocusInfo<'a>> {
        let _span = tracing::trace_span!("next_tab").entered();
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.next_tab_focusable(skip_self).or_else(|| scope.next_tab(true)),
                TabNav::Contained => self.next_tab_focusable(skip_self),
                TabNav::Cycle => self.next_tab_focusable(skip_self).or_else(|| scope.first_tab_descendant()),
                TabNav::Once => scope.next_tab(true),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing SHIFT+TAB from this widget.
    ///
    /// Set `skip_self` to not enter `self`, that is, the focus goes to the previous sibling or previous sibling descendant.
    ///
    /// Returns `None` if the focus does not move to another widget.
    pub fn prev_tab(self, skip_self: bool) -> Option<WidgetFocusInfo<'a>> {
        let _span = tracing::trace_span!("prev_tab").entered();
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.prev_tab_focusable(skip_self).or_else(|| scope.prev_tab(true)),
                TabNav::Contained => self.prev_tab_focusable(skip_self),
                TabNav::Cycle => self.prev_tab_focusable(skip_self).or_else(|| scope.last_tab_descendant()),
                TabNav::Once => scope.prev_tab(true),
            }
        } else {
            None
        }
    }

    fn descendants_skip_directional(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.descendants().filter(move |f| {
            if f.focus_info().skip_directional() {
                TreeFilter::SkipAll
            } else {
                TreeFilter::Include
            }
        })
    }

    fn is_in_direction(from_pt: PxPoint, center: PxPoint, direction: DirectionFn![impl]) -> bool {
        let (a, b, c, d) = direction(from_pt, center);
        let mut is_in_direction = false;

        // for 'up' this is:
        // is above line?
        if a < b {
            // is to the right?
            if c > d {
                // is in the 45º 'frustum'
                // │?╱
                // │╱__
                is_in_direction = c <= d + (b - a);
            } else {
                //  ╲?│
                // __╲│
                is_in_direction = c >= d - (b - a);
            }
        }

        is_in_direction
    }

    fn directional_from_pt(
        self,
        scope: WidgetFocusInfo<'a>,
        from_pt: PxPoint,
        direction: DirectionFn![impl],
        skip_self: bool,
    ) -> Option<WidgetFocusInfo<'a>> {
        let skip_id = self.info.widget_id();
        let parent_id = self.parent().map(|w| w.info.widget_id()).unwrap_or_else(|| skip_id);

        let distance = move |other_pt: PxPoint| {
            let a = (other_pt.x - from_pt.x).0.pow(2);
            let b = (other_pt.y - from_pt.y).0.pow(2);
            a + b
        };

        let mut parent_dist = i32::MAX;
        let mut parent = None;
        let mut sibling_dist = i32::MAX;
        let mut sibling = None;
        let mut other_dist = i32::MAX;
        let mut other = None;

        for w in scope.descendants_skip_directional() {
            if !skip_self || w.info.widget_id() != skip_id {
                let candidate_bounds = w.info.inner_bounds();
                let candidate_center = candidate_bounds.center();

                if Self::is_in_direction(from_pt, candidate_center, &direction) {
                    let dist = distance(candidate_center);

                    let candidate_id = w.info.widget_id();
                    let mut is_parent = None;
                    let mut is_parent = || *is_parent.get_or_insert_with(|| self.ancestors().any(|a| a.info.widget_id() == candidate_id));

                    let mut is_sibling = None;
                    let mut is_sibing = || *is_sibling.get_or_insert_with(|| w.ancestors().any(|a| a.info.widget_id() == parent_id));

                    if dist <= parent_dist && is_parent() {
                        parent_dist = dist;
                        parent = Some(w);
                    } else if dist <= sibling_dist && is_sibing() {
                        sibling_dist = dist;
                        sibling = Some(w);
                    } else if dist <= other_dist && !is_parent() && !is_sibing() {
                        other_dist = dist;
                        other = Some(w);
                    }
                }
            }
        }

        if other_dist <= parent_dist && other_dist <= sibling_dist {
            other
        } else if let Some(sibling) = sibling {
            if skip_self || sibling.info.widget_id() != skip_id {
                Some(sibling)
            } else {
                None
            }
        } else {
            parent
        }
    }

    fn directional_next(self, direction_vals: DirectionFn![impl]) -> Option<WidgetFocusInfo<'a>> {
        self.scope()
            .and_then(|s| self.directional_from_pt(s, self.info.center(), direction_vals, true))
    }

    /// Closest focusable in the same scope above this widget.
    pub fn focusable_up(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![up])
    }

    /// Closest focusable in the same scope below this widget.
    pub fn focusable_down(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![down])
    }

    /// Closest focusable in the same scope to the left of this widget.
    pub fn focusable_left(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![left])
    }

    /// Closest focusable in the same scope to the right of this widget.
    pub fn focusable_right(self) -> Option<WidgetFocusInfo<'a>> {
        self.directional_next(DirectionFn![right])
    }

    /// Widget to focus when pressing the arrow up key from this widget.
    pub fn next_up(self) -> Option<WidgetFocusInfo<'a>> {
        let _span = tracing::trace_span!("next_up").entered();
        self.next_up_from(self.info.center())
    }
    fn next_up_from(self, point: PxPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_up().or_else(|| scope.next_up_from(point)),
                DirectionalNav::Contained => self.focusable_up(),
                DirectionalNav::Cycle => {
                    self.focusable_up().or_else(|| {
                        // next up from the same X but from the bottom segment of scope.
                        let mut from_pt = point;
                        from_pt.y = scope.info.inner_bounds().max().y;
                        self.directional_from_pt(scope, from_pt, DirectionFn![up], false)
                    })
                }
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow right key from this widget.
    pub fn next_right(self) -> Option<WidgetFocusInfo<'a>> {
        let _span = tracing::trace_span!("next_right").entered();
        self.next_right_from(self.info.center())
    }
    fn next_right_from(self, point: PxPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_right().or_else(|| scope.next_right_from(point)),
                DirectionalNav::Contained => self.focusable_right(),
                DirectionalNav::Cycle => self.focusable_right().or_else(|| {
                    // next right from the same Y but from the left segment of scope.
                    let mut from_pt = point;
                    from_pt.x = scope.info.inner_bounds().min().x;
                    self.directional_from_pt(scope, from_pt, DirectionFn![right], false)
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow down key from this widget.
    pub fn next_down(self) -> Option<WidgetFocusInfo<'a>> {
        let _span = tracing::trace_span!("next_down").entered();
        self.next_down_from(self.info.center())
    }
    fn next_down_from(self, point: PxPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_down().or_else(|| scope.next_down_from(point)),
                DirectionalNav::Contained => self.focusable_down(),
                DirectionalNav::Cycle => self.focusable_down().or_else(|| {
                    // next down from the same X but from the top segment of scope.
                    let mut from_pt = point;
                    from_pt.y = scope.info.inner_bounds().min().y;
                    self.directional_from_pt(scope, from_pt, DirectionFn![down], false)
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow left key from this widget.
    pub fn next_left(self) -> Option<WidgetFocusInfo<'a>> {
        let _span = tracing::trace_span!("next_left").entered();
        self.next_left_from(self.info.center())
    }
    fn next_left_from(self, point: PxPoint) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.focusable_left().or_else(|| scope.next_left_from(point)),
                DirectionalNav::Contained => self.focusable_left(),
                DirectionalNav::Cycle => self.focusable_left().or_else(|| {
                    // next left from the same Y but from the right segment of scope.
                    let mut from_pt = point;
                    from_pt.x = scope.info.inner_bounds().max().x;
                    self.directional_from_pt(scope, from_pt, DirectionFn![left], false)
                }),
            }
        } else {
            None
        }
    }

    fn enabled_tab_nav_in_scope(self) -> FocusNavAction {
        // see `prev_tab_focusable` and `next_tab_focusable`

        let both = FocusNavAction::PREV | FocusNavAction::NEXT;

        if self.descendants().any(Self::filter_tab_skip) {
            // can enter
            both
        } else {
            // has next if has logical next with index >= self or has logical prev with index > self.
            // has prev if has logical prev with index <= self or has logical next with index < self.

            let mut nav = FocusNavAction::empty();

            let self_idx = self.focus_info().tab_index();
            for d in self.next_focusables().filter(Self::filter_tab_skip) {
                let idx = d.focus_info().tab_index();

                if idx >= self_idx {
                    nav |= FocusNavAction::NEXT;
                } else {
                    nav |= FocusNavAction::PREV;
                }

                if nav.contains(both) {
                    break;
                }
            }

            if !nav.contains(both) {
                for d in self.prev_focusables().filter(Self::filter_tab_skip) {
                    let idx = d.focus_info().tab_index();

                    if idx <= self_idx {
                        nav |= FocusNavAction::PREV;
                    } else {
                        nav |= FocusNavAction::NEXT;
                    }

                    if nav.contains(both) {
                        break;
                    }
                }
            }

            nav
        }
    }

    fn enabled_tab_nav(self) -> FocusNavAction {
        let _span = tracing::trace_span!("enabled_tab_nav").entered();

        let mut nav = FocusNavAction::empty();

        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => {}
                TabNav::Continue => {
                    nav = self.enabled_tab_nav_in_scope();
                    if !nav.contains(FocusNavAction::PREV | FocusNavAction::NEXT) {
                        nav |= scope.enabled_tab_nav();
                    }
                }
                TabNav::Contained => nav = self.enabled_tab_nav_in_scope(),
                TabNav::Cycle => {
                    if scope.descendants().filter(Self::filter_tab_skip).any(|w| w != self) {
                        nav = FocusNavAction::PREV | FocusNavAction::NEXT;
                    }
                }
                TabNav::Once => nav = scope.enabled_tab_nav(),
            }
        }

        nav
    }

    fn enabled_directional_nav_in_scope(self, scope: WidgetFocusInfo, already_found: FocusNavAction) -> FocusNavAction {
        let mut nav = already_found;

        let center = self.info.center();

        for sibling in scope.descendants_skip_directional() {
            let c = sibling.info.center();

            if !nav.contains(FocusNavAction::UP) && Self::is_in_direction(center, c, DirectionFn![up]) {
                nav |= FocusNavAction::UP;
            }
            if !nav.contains(FocusNavAction::RIGHT) && Self::is_in_direction(center, c, DirectionFn![right]) {
                nav |= FocusNavAction::RIGHT;
            }
            if !nav.contains(FocusNavAction::DOWN) && Self::is_in_direction(center, c, DirectionFn![down]) {
                nav |= FocusNavAction::DOWN;
            }
            if !nav.contains(FocusNavAction::LEFT) && Self::is_in_direction(center, c, DirectionFn![left]) {
                nav |= FocusNavAction::LEFT;
            }

            if nav.contains(FocusNavAction::DIRECTIONAL) {
                break;
            }
        }

        nav
    }
    fn enabled_directional_nav(self, already_found: FocusNavAction) -> FocusNavAction {
        let _span = tracing::trace_span!("enabled_directional_nav").entered();

        let mut nav = already_found;

        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();

            match scope_info.directional_nav() {
                DirectionalNav::None => {}
                DirectionalNav::Continue => {
                    nav = self.enabled_directional_nav_in_scope(scope, nav);
                    if !nav.contains(FocusNavAction::DIRECTIONAL) {
                        nav |= scope.enabled_directional_nav(nav)
                    }
                }
                DirectionalNav::Contained => nav = self.enabled_directional_nav_in_scope(scope, nav),
                DirectionalNav::Cycle => {
                    if scope.descendants_skip_directional().any(|w| w != self) {
                        nav = FocusNavAction::DIRECTIONAL;
                    }
                }
            }
        }

        nav
    }

    /// Focus navigation actions that can move the focus away from this item.
    pub fn enabled_nav(self) -> FocusNavAction {
        let _span = tracing::trace_span!("enabled_nav").entered();

        let mut actions = FocusNavAction::empty();

        actions.set(FocusNavAction::EXIT, self.parent().is_some() || self.is_alt_scope());
        actions.set(FocusNavAction::ENTER, self.descendants().next().is_some());

        actions |= self.enabled_tab_nav();
        actions |= self.enabled_directional_nav(FocusNavAction::empty());

        actions.set(FocusNavAction::ALT, self.in_alt_scope() || self.alt_scope().is_some());

        actions
    }

    /// Gets the focusable descendant with center closest to `from_pt`.
    pub fn closest_descendant(self, from_pt: PxPoint) -> Option<WidgetFocusInfo<'a>> {
        let distance = move |other_pt: PxPoint| {
            let a = (other_pt.x - from_pt.x).0.pow(2);
            let b = (other_pt.y - from_pt.y).0.pow(2);
            a + b
        };

        let mut candidate_dist = i32::MAX;
        let mut candidate = None;

        for c in self.descendants() {
            let dist = distance(c.info.center());
            if dist <= candidate_dist {
                candidate_dist = dist;
                candidate = Some(c);
            }
        }

        candidate
    }
}

/// Focus metadata associated with a widget info tree.
#[derive(Debug, Clone, Copy)]
pub enum FocusInfo {
    /// The widget is not focusable.
    NotFocusable,
    /// The widget is focusable as a single item.
    Focusable {
        /// Tab index of the widget.
        tab_index: TabIndex,
        /// If the widget is skipped during directional navigation from outside.
        skip_directional: bool,
    },
    /// The widget is a focusable focus scope.
    FocusScope {
        /// Tab index of the widget.
        tab_index: TabIndex,
        /// If the widget is skipped during directional navigation from outside.
        skip_directional: bool,
        /// Tab navigation inside the focus scope.
        tab_nav: TabNav,
        /// Directional navigation inside the focus scope.
        directional_nav: DirectionalNav,
        /// Behavior of the widget when receiving direct focus.
        on_focus: FocusScopeOnFocus,
        /// If this scope is focused when the ALT key is pressed.
        alt: bool,
    },
}

/// Behavior of a focus scope when it receives direct focus.
#[derive(Clone, Copy, Eq, PartialEq)]
pub enum FocusScopeOnFocus {
    /// Just focus the scope widget.
    Widget,
    /// Focus the first descendant considering the TAB index, if the scope has no descendants
    /// behaves like [`Widget`](Self::Widget).
    ///
    /// Focus the last descendant if the focus is *reversing* in, e.g. in a SHIFT+TAB action.
    FirstDescendant,
    /// Focus the descendant that was last focused before focus moved out of the scope. If the
    /// scope cannot return focus, behaves like [`FirstDescendant`](Self::FirstDescendant).
    LastFocused,
}
impl fmt::Debug for FocusScopeOnFocus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "FocusScopeOnFocus::")?;
        }
        match self {
            FocusScopeOnFocus::Widget => write!(f, "Widget"),
            FocusScopeOnFocus::FirstDescendant => write!(f, "FirstDescendant"),
            FocusScopeOnFocus::LastFocused => write!(f, "LastFocused"),
        }
    }
}
impl Default for FocusScopeOnFocus {
    /// [`FirstDescendant`](Self::FirstDescendant)
    fn default() -> Self {
        FocusScopeOnFocus::FirstDescendant
    }
}

impl FocusInfo {
    /// If is focusable or a focus scope.
    pub fn is_focusable(self) -> bool {
        !matches!(self, FocusInfo::NotFocusable)
    }

    /// If is a focus scope.
    pub fn is_scope(self) -> bool {
        matches!(self, FocusInfo::FocusScope { .. })
    }

    /// If is an ALT focus scope.
    pub fn is_alt_scope(self) -> bool {
        match self {
            FocusInfo::FocusScope { alt, .. } => alt,
            _ => false,
        }
    }

    /// Tab navigation mode.
    ///
    /// | Variant                   | Returns                                 |
    /// |---------------------------|-----------------------------------------|
    /// | Focus scope               | Associated value, default is `Continue` |
    /// | Focusable                 | `TabNav::Continue`                      |
    /// | Not-Focusable             | `TabNav::None`                          |
    pub fn tab_nav(self) -> TabNav {
        match self {
            FocusInfo::FocusScope { tab_nav, .. } => tab_nav,
            FocusInfo::Focusable { .. } => TabNav::Continue,
            FocusInfo::NotFocusable => TabNav::None,
        }
    }

    /// Directional navigation mode.
    ///
    /// | Variant                   | Returns                             |
    /// |---------------------------|-------------------------------------|
    /// | Focus scope               | Associated value, default is `None` |
    /// | Focusable                 | `DirectionalNav::Continue`          |
    /// | Not-Focusable             | `DirectionalNav::None`              |
    pub fn directional_nav(self) -> DirectionalNav {
        match self {
            FocusInfo::FocusScope { directional_nav, .. } => directional_nav,
            FocusInfo::Focusable { .. } => DirectionalNav::Continue,
            FocusInfo::NotFocusable => DirectionalNav::None,
        }
    }

    /// Tab navigation index.
    ///
    /// | Variant           | Returns                                       |
    /// |-------------------|-----------------------------------------------|
    /// | Focusable & Scope | Associated value, default is `TabIndex::AUTO` |
    /// | Not-Focusable     | `TabIndex::SKIP`                              |
    pub fn tab_index(self) -> TabIndex {
        match self {
            FocusInfo::Focusable { tab_index, .. } => tab_index,
            FocusInfo::FocusScope { tab_index, .. } => tab_index,
            FocusInfo::NotFocusable => TabIndex::SKIP,
        }
    }

    /// If directional navigation skips over this widget.
    ///
    /// | Variant           | Returns                                       |
    /// |-------------------|-----------------------------------------------|
    /// | Focusable & Scope | Associated value, default is `false`          |
    /// | Not-Focusable     | `true`                                        |
    pub fn skip_directional(self) -> bool {
        match self {
            FocusInfo::Focusable { skip_directional, .. } => skip_directional,
            FocusInfo::FocusScope { skip_directional, .. } => skip_directional,
            FocusInfo::NotFocusable => true,
        }
    }

    /// Focus scope behavior when it receives direct focus.
    ///
    /// | Variant                   | Returns                                                           |
    /// |---------------------------|-------------------------------------------------------------------|
    /// | Scope                     | Associated value, default is `FocusScopeOnFocus::FirstDescendant` |
    /// | Focusable & Not-Focusable | `FocusScopeOnFocus::Self_`                                        |
    pub fn scope_on_focus(self) -> FocusScopeOnFocus {
        match self {
            FocusInfo::FocusScope { on_focus, .. } => on_focus,
            _ => FocusScopeOnFocus::Widget,
        }
    }
}

/// Builder for [`FocusInfo`] accessible in a [`WidgetInfoBuilder`].
///
/// Use the [`FocusInfoKey`] to access the builder for the widget state in a widget info.
///
/// [`WidgetInfoBuilder`]: crate::widget_info::WidgetInfoBuilder
#[derive(Default)]
pub struct FocusInfoBuilder {
    /// If the widget is focusable and the value was explicitly set.
    pub focusable: Option<bool>,

    /// If the widget is a focus scope and the value was explicitly set.
    pub scope: Option<bool>,
    /// If the widget is an ALT focus scope when it is a focus scope.
    pub alt_scope: bool,
    /// When the widget is a focus scope, its behavior on receiving direct focus.
    pub on_focus: FocusScopeOnFocus,

    /// Widget TAB index and if the index was explicitly set.
    pub tab_index: Option<TabIndex>,

    /// TAB navigation within this widget, if set turns the widget into a focus scope.
    pub tab_nav: Option<TabNav>,
    /// Directional navigation within this widget, if set turns the widget into a focus scope.
    pub directional_nav: Option<DirectionalNav>,

    /// If directional navigation skips over this widget.
    pub skip_directional: Option<bool>,
}
impl FocusInfoBuilder {
    /// Build a [`FocusInfo`] from the collected configuration in `self`.
    ///
    /// The widget is not focusable nor a focus scope if it set [`focusable`](Self::focusable) to `false`.
    ///
    /// The widget is a *focus scope* if it set [`scope`](Self::scope) to `true` **or** if it set [`tab_nav`](Self::tab_nav) or
    /// [`directional_nav`](Self::directional_nav) and did not set [`scope`](Self::scope) to `false`.
    ///
    /// The widget is *focusable* if it set [`focusable`](Self::focusable) to `true` **or** if it set the [`tab_index`](Self::tab_index).
    ///
    /// The widget is not focusable if it did not set any of the members mentioned.
    ///
    /// ## Tab Index
    ///
    /// If the [`tab_index`](Self::tab_index) was not set but the widget is focusable or a focus scope, the [`TabIndex::AUTO`]
    /// is used for the widget.
    ///
    /// ## Skip Directional
    ///
    /// If the [`skip_directional`](Self::skip_directional) was not set but the widget is focusable or a focus scope, it is
    /// set to `false` for the widget.
    ///
    /// ## Focus Scope
    ///
    /// If the widget is a focus scope, it is configured using [`alt_scope`](Self::alt_scope) and [`on_focus`](Self::on_focus).
    /// If the widget is not a scope these members are ignored.
    ///
    /// ### Tab Navigation
    ///
    /// If [`tab_nav`](Self::tab_nav) is not set but the widget is a focus scope, [`TabNav::Continue`] is used.
    ///
    /// ### Directional Navigation
    ///
    /// If [`directional_nav`](Self::directional_nav) is not set but the widget is a focus scope, [`DirectionalNav::Continue`] is used.
    pub fn build(&self) -> FocusInfo {
        match (self.focusable, self.scope, self.tab_index, self.tab_nav, self.directional_nav) {
            // Set as not focusable.
            (Some(false), _, _, _, _) => FocusInfo::NotFocusable,

            // Set as focus scope and not set as not focusable
            // or set tab navigation and did not set as not focus scope
            // or set directional navigation and did not set as not focus scope.
            (_, Some(true), idx, tab, dir) | (_, None, idx, tab @ Some(_), dir) | (_, None, idx, tab, dir @ Some(_)) => {
                FocusInfo::FocusScope {
                    tab_index: idx.unwrap_or(TabIndex::AUTO),
                    skip_directional: self.skip_directional.unwrap_or_default(),
                    tab_nav: tab.unwrap_or(TabNav::Continue),
                    directional_nav: dir.unwrap_or(DirectionalNav::Continue),
                    alt: self.alt_scope,
                    on_focus: self.on_focus,
                }
            }

            // Set as focusable and was not focus scope
            // or set tab index and was not focus scope and did not set as not focusable.
            (Some(true), _, idx, _, _) | (_, _, idx @ Some(_), _, _) => FocusInfo::Focusable {
                tab_index: idx.unwrap_or(TabIndex::AUTO),
                skip_directional: self.skip_directional.unwrap_or_default(),
            },

            _ => FocusInfo::NotFocusable,
        }
    }
}
