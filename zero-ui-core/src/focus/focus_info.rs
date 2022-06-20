use crate::{
    context::*,
    units::*,
    var::impl_from_and_into_var,
    widget_info::{DescendantFilter, Interactivity, Visibility, WidgetInfo, WidgetInfoTree, WidgetPath},
    window::FocusIndicator,
    WidgetId,
};
use std::fmt;

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
    /// Tab moves into the scope but does not move the focus inside the scope.
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
    /// Arrows does not move the focus inside the scope.
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
/// See [`Focus::focus`] for details.
pub struct FocusRequest {
    /// Where to move the focus.
    pub target: FocusTarget,
    /// If the widget should visually indicate that it has keyboard focus.
    pub highlight: bool,

    /// If the window should be focused even if another app has focus. By default the window
    /// is only focused if the app has keyboard focus in any of the open windows, if this is enabled
    /// a [`Windows::focus`] request is always made, potentially stealing keyboard focus from another app
    /// and disrupting the user.
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
    /// New [`FocusTarget::Child`] request.
    pub fn child(highlight: bool) -> Self {
        Self::new(FocusTarget::Child, highlight)
    }
    /// New [`FocusTarget::Parent`] request.
    pub fn parent(highlight: bool) -> Self {
        Self::new(FocusTarget::Parent, highlight)
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
    /// New [`FocusTarget::EscapeAlt`] request.
    pub fn escape_alt(highlight: bool) -> Self {
        Self::new(FocusTarget::EscapeAlt, highlight)
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
    Child,
    /// Move focus to the first focusable ancestor of the current focus, or to first in screen.
    Parent,

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

    /// Move focus to the current widget ALT scope.
    Alt,
    /// Move focus back from ALT scope.
    EscapeAlt,
}

bitflags! {
    /// Represents the [`FocusTarget`] actions that move focus from the current focused widget.
    pub struct FocusNavAction: u16 {
        /// [`FocusTarget::Child`]
        const CHILD =      0b0000_0000_0001;
        /// [`FocusTarget::Parent`]
        const PARENT =     0b0000_0000_0010;

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
        /// [`FocusTarget::EscapeAlt`]
        const ESCAPE_ALT = 0b0010_0000_0000;
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
    /// [`focus_disabled_widgets`]: Focus::focus_disabled_widgets
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

        for w in root.filter_descendants(|_| DescendantFilter::SkipDescendants) {
            let weight = w.info.prev_siblings().count() + w.info.ancestors().count();
            if weight < candidate_weight {
                candidate = Some(w);
                candidate_weight = weight;
            }
        }

        candidate
    }

    /// Reference to the widget in the tree, if it is present and is focusable.
    pub fn find(&self, widget_id: WidgetId) -> Option<WidgetFocusInfo> {
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
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.find(widget_id).is_some()
    }
}

/// [`WidgetInfo`] extensions that build a [`WidgetFocusInfo`].
pub trait WidgetInfoFocusExt<'a> {
    /// Wraps the [`WidgetInfo`] in a [`WidgetFocusInfo`] even if it is not focusable.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details on the second parameter.
    #[allow(clippy::wrong_self_convention)] // WidgetFocusInfo is a reference wrapper.
    fn as_focus_info(self, focus_disabled_widgets: bool) -> WidgetFocusInfo<'a>;

    /// Returns a wrapped [`WidgetFocusInfo`] if the [`WidgetInfo`] is focusable.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details on the second parameter.
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
    /// [`DISABLED`]: Interactivity::DISABLED
    /// [`focus_disabled_widgets`]: Focus::focus_disabled_widgets
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

    /// Gets the [`scope`](Self::scope) and the widgets from the scope to `self`.
    fn scope_with_path(self) -> Option<(WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>)> {
        let mut path = vec![];
        for i in self.info.ancestors() {
            let i = i.as_focus_info(self.focus_disabled_widgets());
            if i.is_scope() {
                path.reverse();
                return Some((i, path));
            } else {
                path.push(i);
            }
        }
        None
    }

    /// Reference the ALT focus scope *closest* with the current widget.
    ///
    /// # Closest Alt Scope
    ///
    /// a - If `self` is already an ALT scope or is in one, moves to a sibling ALT scope, nested ALT scopes are ignored.
    /// b - If `self` is a normal scope, moves to the first descendant ALT scope, otherwise..
    /// c - Recursively searches for an ALT scope sibling up the scope tree.
    pub fn alt_scope(self) -> Option<WidgetFocusInfo<'a>> {
        if self.in_alt_scope() {
            // We do not allow nested alt scopes, search for sibling focus scope.
            return self.scopes().find(|s| !s.is_alt_scope()).and_then(|s| s.alt_scope_query(s));
        } else if self.is_scope() {
            // if we are a normal scope, search for an inner ALT scope first.
            let r = self.descendants().find(|w| w.focus_info().is_alt_scope());
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
                .filter_descendants(|w| {
                    if w == skip {
                        DescendantFilter::SkipAll
                    } else {
                        DescendantFilter::Include
                    }
                })
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
    pub fn descendants(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.descendants().focusable(self.focus_disabled_widgets())
    }

    /// Iterator over all focusable widgets contained by this widget filtered by the `filter` closure.
    ///
    /// If `skip` returns `true` the widget and all its descendants are skipped.
    pub fn filter_descendants(
        self,
        mut filter: impl FnMut(WidgetFocusInfo<'a>) -> DescendantFilter,
    ) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info
            .filter_descendants(move |info| {
                if let Some(focusable) = info.as_focusable(self.focus_disabled_widgets()) {
                    filter(focusable)
                } else {
                    DescendantFilter::Skip
                }
            })
            .map(move |info| info.as_focus_info(self.focus_disabled_widgets()))
    }

    /// Descendants sorted by TAB index.
    ///
    /// [`SKIP`](TabIndex::SKIP) focusable items and its descendants are not included.
    pub fn tab_descendants(self) -> Vec<WidgetFocusInfo<'a>> {
        self.filter_tab_descendants(|f| {
            if f.focus_info().tab_index().is_skip() {
                DescendantFilter::SkipAll
            } else {
                DescendantFilter::Include
            }
        })
    }

    /// Like [`tab_descendants`](Self::tab_descendants) but you can customize what items are skipped.
    pub fn filter_tab_descendants(self, filter: impl Fn(WidgetFocusInfo) -> DescendantFilter) -> Vec<WidgetFocusInfo<'a>> {
        let mut vec: Vec<_> = self.filter_descendants(filter).collect();
        vec.sort_by_key(|f| f.focus_info().tab_index());
        vec
    }

    /// First descendant considering TAB index.
    pub fn first_tab_descendant(self) -> Option<WidgetFocusInfo<'a>> {
        self.tab_descendants().first().copied()
    }

    /// Last descendant considering TAB index.
    pub fn last_tab_descendant(self) -> Option<WidgetFocusInfo<'a>> {
        self.tab_descendants().last().copied()
    }

    /// Iterator over all focusable widgets in the same scope after this widget.
    pub fn next_focusables(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();
        self.scope()
            .into_iter()
            .flat_map(|s| s.descendants())
            .skip_while(move |f| f.info.widget_id() != self_id)
            .skip(1)
    }

    /// Next focusable in the same scope after this widget.
    pub fn next_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_focusables().next()
    }

    /// Next focusable in the same scope after this widget respecting the TAB index.
    ///
    /// If `self` is `TabIndex::SKIP`returns the next non-skip focusable in the same scope after this widget.
    ///
    /// If `skip_self` is `true`, does not include widgets inside `self`.
    ///
    /// If `self` is the last item in scope returns the sorted descendants of the parent scope.
    pub fn next_tab_focusable(self, skip_self: bool) -> Result<WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>> {
        let self_index = self.focus_info().tab_index();

        // TAB siblings in the scope.
        //  - If `skip_self` excludes our own branch and SKIP branches.
        //  - If not `skip_self` includes our own branch even if it is SKIP but excludes all other SKIP branches.
        let mut siblings = self.siblings_skip_self(skip_self);

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes to next in widget tree.
            return self
                .info
                .next_siblings()
                .map(|s| s.as_focus_info(self.focus_disabled_widgets()))
                .find(|s| s.focus_info().tab_index() != TabIndex::SKIP)
                .ok_or(siblings);
        }

        // binary search the same tab index gets any of the items with the same tab index.
        let i_same = siblings.binary_search_by_key(&self_index, |f| f.focus_info().tab_index()).unwrap();
        // so we do a linear search before and after to find `self`.

        // before
        for i in (0..=i_same).rev() {
            if siblings[i] == self {
                return if i == siblings.len() - 1 {
                    // we are the last item.
                    Err(siblings)
                } else {
                    // next
                    Ok(siblings.swap_remove(i + 1))
                };
            } else if siblings[i].focus_info().tab_index() != self_index {
                // did not find `self` before `i_same`
                break;
            }
        }

        // after
        for i in i_same..siblings.len() {
            if siblings[i] == self {
                return if i == siblings.len() - 1 {
                    // we are the last item.
                    Err(siblings)
                } else {
                    // next
                    Ok(siblings.swap_remove(i + 1))
                };
            }
        }

        Err(siblings)
    }

    /// Iterator over all focusable widgets in the same scope before this widget in reverse.
    pub fn prev_focusables(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();

        let mut prev: Vec<_> = self
            .scope()
            .into_iter()
            .flat_map(|s| s.descendants())
            .take_while(move |f| f.info.widget_id() != self_id)
            .collect();

        prev.reverse();

        prev.into_iter()
    }

    /// Previous focusable in the same scope before this widget.
    pub fn prev_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();

        self.scope()
            .and_then(move |s| s.descendants().take_while(move |f| f.info.widget_id() != self_id).last())
    }

    fn siblings_skip_self(self, skip_self: bool) -> Vec<WidgetFocusInfo<'a>> {
        self.scope_with_path()
            .map(|(scope, path)| {
                scope.filter_tab_descendants(|i| {
                    if skip_self && i == self {
                        // does not skip `self`, but skips all inside `self`
                        DescendantFilter::SkipDescendants
                    } else if i.focus_info().tab_index().is_skip() {
                        // skip only if is not a parent of `self`
                        if path.iter().all(|&p| p != i) {
                            DescendantFilter::SkipAll
                        } else {
                            DescendantFilter::Include
                        }
                    } else {
                        DescendantFilter::Include
                    }
                })
            })
            .unwrap_or_default()
    }

    /// Previous focusable in the same scope before this widget respecting the TAB index.
    ///
    /// If `self` is `TabIndex::SKIP` or `skip_self` is set returns the previous non-skip
    /// focusable in the same scope before this widget.
    ///
    /// If `self` is the first item in scope returns the sorted descendants of the parent scope.
    pub fn prev_tab_focusable(self, skip_self: bool) -> Result<WidgetFocusInfo<'a>, Vec<WidgetFocusInfo<'a>>> {
        let self_index = self.focus_info().tab_index();

        let mut siblings = self.siblings_skip_self(skip_self);

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes to prev in widget tree.
            return self
                .info
                .prev_siblings()
                .map(|s| s.as_focus_info(self.focus_disabled_widgets()))
                .find(|s| s.focus_info().tab_index() != TabIndex::SKIP)
                .ok_or(siblings);
        }

        // binary search the same tab index gets any of the items with the same tab index.
        let i_same = siblings.binary_search_by_key(&self_index, |f| f.focus_info().tab_index()).unwrap();

        // before
        for i in (0..=i_same).rev() {
            if siblings[i] == self {
                return if i == 0 {
                    // we are the first item.
                    Err(siblings)
                } else {
                    // prev
                    Ok(siblings.swap_remove(i - 1))
                };
            } else if siblings[i].focus_info().tab_index() != self_index {
                // did not find `self` before `i_same`
                break;
            }
        }

        // after
        for i in i_same..siblings.len() {
            if siblings[i] == self {
                return if i == 0 {
                    // we are the first item.
                    Err(siblings)
                } else {
                    // prev
                    Ok(siblings.swap_remove(i - 1))
                };
            }
        }

        Err(siblings)
    }

    /// Widget to focus when pressing TAB from this widget.
    ///
    /// Set `skip_self` to not enter `self`, that is, the focus goes to the next sibling or next sibling descendant.
    ///
    /// Returns `None` if the focus does not move to another widget.
    pub fn next_tab(self, skip_self: bool) -> Option<WidgetFocusInfo<'a>> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.next_tab_focusable(skip_self).ok().or_else(|| scope.next_tab(true)),
                TabNav::Contained => self.next_tab_focusable(skip_self).ok(),
                TabNav::Cycle => self
                    .next_tab_focusable(skip_self)
                    .or_else(|sorted_siblings| {
                        if let Some(first) = sorted_siblings.into_iter().find(|f| f.focus_info().tab_index() != TabIndex::SKIP) {
                            if first == self {
                                Err(())
                            } else {
                                Ok(first)
                            }
                        } else {
                            Err(())
                        }
                    })
                    .ok(),
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
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.tab_nav() {
                TabNav::None => None,
                TabNav::Continue => self.prev_tab_focusable(skip_self).ok().or_else(|| scope.prev_tab(true)),
                TabNav::Contained => self.prev_tab_focusable(skip_self).ok(),
                TabNav::Cycle => self
                    .prev_tab_focusable(skip_self)
                    .or_else(|sorted_siblings| {
                        if let Some(last) = sorted_siblings.into_iter().rfind(|f| f.focus_info().tab_index() != TabIndex::SKIP) {
                            if last == self {
                                Err(())
                            } else {
                                Ok(last)
                            }
                        } else {
                            Err(())
                        }
                    })
                    .ok(),
                TabNav::Once => scope.prev_tab(true),
            }
        } else {
            None
        }
    }

    fn descendants_skip_directional(self, also_skip: Option<WidgetFocusInfo<'a>>) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.filter_descendants(move |f| {
            if also_skip == Some(f) || f.focus_info().skip_directional() {
                DescendantFilter::SkipAll
            } else {
                DescendantFilter::Include
            }
        })
    }

    fn directional_from_pt(
        self,
        scope: WidgetFocusInfo<'a>,
        from_pt: PxPoint,
        direction: DirectionFn![impl],
        skip_descendants: bool,
    ) -> Option<WidgetFocusInfo<'a>> {
        let skip_id = self.info.widget_id();

        let distance = move |other_pt: PxPoint| {
            let a = (other_pt.x - from_pt.x).0.pow(2);
            let b = (other_pt.y - from_pt.y).0.pow(2);
            a + b
        };

        let mut candidate_dist = i32::MAX;
        let mut candidate = None;

        for w in scope.descendants_skip_directional(if skip_descendants { Some(self) } else { None }) {
            if w.info.widget_id() != skip_id {
                let candidate_center = w.info.center();

                let (a, b, c, d) = direction(from_pt, candidate_center);
                let mut is_in_direction = false;

                // for 'up' this is:
                // is above line?
                if a <= b {
                    // is to the right?
                    if c >= d {
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

                if is_in_direction {
                    let dist = distance(candidate_center);
                    if dist < candidate_dist {
                        candidate = Some(w);
                        candidate_dist = dist;
                    }
                }
            }
        }

        candidate
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

    /// Focus navigation actions that can move the focus away from this item.
    pub fn enabled_nav(self) -> FocusNavAction {
        let mut actions = FocusNavAction::empty();
        actions.set(FocusNavAction::PARENT, self.parent().is_some());
        actions.set(FocusNavAction::CHILD, self.descendants().next().is_some());

        actions.set(FocusNavAction::NEXT, self.next_tab(false).is_some());
        actions.set(FocusNavAction::PREV, self.prev_tab(false).is_some());

        actions.set(FocusNavAction::UP, self.next_up().is_some());
        actions.set(FocusNavAction::RIGHT, self.next_right().is_some());
        actions.set(FocusNavAction::DOWN, self.next_down().is_some());
        actions.set(FocusNavAction::LEFT, self.next_left().is_some());

        actions.set(FocusNavAction::ALT, self.alt_scope().is_some());
        actions.set(FocusNavAction::ESCAPE_ALT, self.in_alt_scope());

        actions
    }
}

/// Filter-maps an iterator of [`WidgetInfo`] to [`WidgetFocusInfo`].
pub trait IterFocusable<'a, I: Iterator<Item = WidgetInfo<'a>>> {
    /// Returns an iterator of only the focusable widgets.
    ///
    /// See the [`Focus::focus_disabled_widgets`] config for more details on the second parameter.
    fn focusable(
        self,
        focus_disabled_widgets: bool,
    ) -> std::iter::FilterMap<I, Box<dyn FnMut(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>>>;
}
impl<'a, I: Iterator<Item = WidgetInfo<'a>>> IterFocusable<'a, I> for I {
    fn focusable(
        self,
        focus_disabled_widgets: bool,
    ) -> std::iter::FilterMap<I, Box<dyn FnMut(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>>> {
        self.filter_map(Box::new(move |i| i.as_focusable(focus_disabled_widgets)))
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
