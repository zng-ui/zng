use std::fmt;
use std::sync::atomic::Ordering::Relaxed;

use atomic::Atomic;
use parking_lot::Mutex;
use zng_app::{
    widget::{
        WidgetId,
        info::{TreeFilter, Visibility, WidgetInfo, WidgetInfoBuilder, WidgetInfoTree, WidgetPath},
    },
    window::WindowId,
};
use zng_ext_window::NestedWindowWidgetInfoExt;
use zng_layout::unit::{DistanceKey, Orientation2D, Px, PxBox, PxPoint, PxRect, PxSize};
use zng_state_map::{StateId, static_id};
use zng_unique_id::IdSet;
use zng_var::impl_from_and_into_var;
use zng_view_api::window::FocusIndicator;

use zng_app::widget::info::iter as w_iter;

use super::iter::IterFocusableExt;

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

    /// Last possible widget index.
    pub const LAST: TabIndex = TabIndex(u32::MAX - 1);

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
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
enum TabIndexSerde<'s> {
    Named(&'s str),
    Unnamed(u32),
}
impl serde::Serialize for TabIndex {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if serializer.is_human_readable() {
            let name = if self.is_auto() {
                Some("AUTO")
            } else if self.is_skip() {
                Some("SKIP")
            } else {
                None
            };
            if let Some(name) = name {
                return TabIndexSerde::Named(name).serialize(serializer);
            }
        }
        TabIndexSerde::Unnamed(self.0).serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for TabIndex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error;

        match TabIndexSerde::deserialize(deserializer)? {
            TabIndexSerde::Named(name) => match name {
                "AUTO" => Ok(TabIndex::AUTO),
                "SKIP" => Ok(TabIndex::SKIP),
                unknown => Err(D::Error::unknown_variant(unknown, &["AUTO", "SKIP"])),
            },
            TabIndexSerde::Unnamed(i) => Ok(TabIndex(i)),
        }
    }
}

/// Tab navigation configuration of a focus scope.
///
/// See the [module level](crate::focus#tab-navigation) for an overview of tab navigation.
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DirectionalNav {
    /// Arrows can move into the scope, but does not move the focus inside the scope.
    None,
    /// Arrows move the focus through the scope continuing out of the edges.
    Continue,
    /// Arrows move the focus inside the scope only, stops at the edges.
    Contained,
    /// Arrows move the focus inside the scope only, cycles back to opposite edges.
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

/// Focus change request.
///
/// See [`FOCUS`] for details.
///
/// [`FOCUS`]: crate::focus::FOCUS::focus
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct FocusRequest {
    /// Where to move the focus.
    pub target: FocusTarget,
    /// If the widget should visually indicate that it has keyboard focus.
    pub highlight: bool,

    /// If the window should be focused even if another app has focus. By default the window
    /// is only focused if the app has keyboard focus in any of the open windows, if this is enabled
    /// a [`WINDOWS.focus`] request is always made, potentially stealing keyboard focus from another app
    /// and disrupting the user.
    ///
    /// [`WINDOWS.focus`]: zng_ext_window::WINDOWS::focus
    pub force_window_focus: bool,

    /// Focus indicator to set on the target window if the app does not have keyboard focus and
    /// `force_window_focus` is disabled.
    ///
    /// The [`focus_indicator`] of the window is set and the request is processed after the window receives focus,
    /// or it is canceled if another focus request is made.
    ///
    /// [`focus_indicator`]: zng_ext_window::WindowVars::focus_indicator
    pub window_indicator: Option<FocusIndicator>,
}

impl FocusRequest {
    /// New request from target and highlight.
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
        Self::new(FocusTarget::Direct { target: widget_id }, highlight)
    }
    /// New [`FocusTarget::DirectOrExit`] request.
    pub fn direct_or_exit(widget_id: WidgetId, navigation_origin: bool, highlight: bool) -> Self {
        Self::new(
            FocusTarget::DirectOrExit {
                target: widget_id,
                navigation_origin,
            },
            highlight,
        )
    }
    /// New [`FocusTarget::DirectOrEnter`] request.
    pub fn direct_or_enter(widget_id: WidgetId, navigation_origin: bool, highlight: bool) -> Self {
        Self::new(
            FocusTarget::DirectOrEnter {
                target: widget_id,
                navigation_origin,
            },
            highlight,
        )
    }
    /// New [`FocusTarget::DirectOrRelated`] request.
    pub fn direct_or_related(widget_id: WidgetId, navigation_origin: bool, highlight: bool) -> Self {
        Self::new(
            FocusTarget::DirectOrRelated {
                target: widget_id,
                navigation_origin,
            },
            highlight,
        )
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FocusTarget {
    /// Move focus to widget.
    Direct {
        /// Focusable widget.
        target: WidgetId,
    },
    /// Move focus to the widget if it is focusable or to the first focusable ancestor.
    DirectOrExit {
        /// Maybe focusable widget.
        target: WidgetId,
        /// If `true` the `target` becomes the [`navigation_origin`] when the first focusable ancestor
        /// is focused because the `target` is not focusable.
        ///
        /// [`navigation_origin`]: crate::focus::FOCUS::navigation_origin
        navigation_origin: bool,
    },
    /// Move focus to the widget if it is focusable or to first focusable descendant.
    DirectOrEnter {
        /// Maybe focusable widget.
        target: WidgetId,
        /// If `true` the `target` becomes the [`navigation_origin`] when the first focusable descendant
        /// is focused because the `target` is not focusable.
        ///
        /// [`navigation_origin`]: crate::focus::FOCUS::navigation_origin
        navigation_origin: bool,
    },
    /// Move focus to the widget if it is focusable, or to the first focusable descendant or
    /// to the first focusable ancestor.
    DirectOrRelated {
        /// Maybe focusable widget.
        target: WidgetId,
        /// If `true` the `target` becomes the [`navigation_origin`] when the first focusable relative
        /// is focused because the `target` is not focusable.
        ///
        /// [`navigation_origin`]: crate::focus::FOCUS::navigation_origin
        navigation_origin: bool,
    },

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
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        const DIRECTIONAL = FocusNavAction::UP.bits() | FocusNavAction::RIGHT.bits() | FocusNavAction::DOWN.bits() | FocusNavAction::LEFT.bits();
    }
}

bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub(super) struct FocusMode: u8 {
        /// Allow focus in disabled widgets.
        const DISABLED = 1;
        /// Allow focus in hidden widgets.
        const HIDDEN = 2;
    }
}
impl FocusMode {
    pub fn new(focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> Self {
        let mut mode = FocusMode::empty();
        mode.set(FocusMode::DISABLED, focus_disabled_widgets);
        mode.set(FocusMode::HIDDEN, focus_hidden_widgets);
        mode
    }
}

/// A [`WidgetInfoTree`] wrapper for querying focus info out of the widget tree.
///
/// [`WidgetInfoTree`]: zng_app::widget::info::WidgetInfoTree
#[derive(Clone, Debug)]
pub struct FocusInfoTree {
    tree: WidgetInfoTree,
    mode: FocusMode,
}
impl FocusInfoTree {
    /// Wrap a `widget_info` reference to enable focus info querying.
    ///
    /// See the [`FOCUS.focus_disabled_widgets`] and [`FOCUS.focus_hidden_widgets`] config for more details on the parameters.
    ///
    /// [`FOCUS.focus_disabled_widgets`]: crate::focus::FOCUS::focus_disabled_widgets
    /// [`FOCUS.focus_hidden_widgets`]: crate::focus::FOCUS::focus_hidden_widgets
    pub fn new(tree: WidgetInfoTree, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> Self {
        FocusInfoTree {
            tree,
            mode: FocusMode::new(focus_disabled_widgets, focus_hidden_widgets),
        }
    }

    /// Full widget info.
    pub fn tree(&self) -> &WidgetInfoTree {
        &self.tree
    }

    /// If [`DISABLED`] widgets are focusable in this tree.
    ///
    /// See the [`FOCUS.focus_disabled_widgets`] config for more details.
    ///
    /// [`DISABLED`]: zng_app::widget::info::Interactivity::DISABLED
    /// [`FOCUS.focus_disabled_widgets`]: crate::focus::FOCUS::focus_disabled_widgets
    pub fn focus_disabled_widgets(&self) -> bool {
        self.mode.contains(FocusMode::DISABLED)
    }

    /// If [`Hidden`] widgets are focusable in this tree.
    ///
    /// See the [`FOCUS.focus_hidden_widgets`] config for more details.
    ///
    /// [`Hidden`]: Visibility::Hidden
    /// [`FOCUS.focus_hidden_widgets`]: crate::focus::FOCUS::focus_hidden_widgets
    pub fn focus_hidden_widgets(&self) -> bool {
        self.mode.contains(FocusMode::HIDDEN)
    }

    /// Reference to the root widget in the tree.
    ///
    /// The root is usually a focusable focus scope but it may not be. This
    /// is the only method that returns a [`WidgetFocusInfo`] that may not be focusable.
    pub fn root(&self) -> WidgetFocusInfo {
        WidgetFocusInfo {
            info: self.tree.root(),
            mode: self.mode,
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

        for w in root.descendants().tree_filter(|_| TreeFilter::SkipDescendants) {
            let weight = w.info.prev_siblings().count() + w.info.ancestors().count();
            if weight < candidate_weight {
                candidate = Some(w);
                candidate_weight = weight;
            }
        }

        candidate
    }

    /// Reference to the widget in the tree, if it is present and is focusable.
    pub fn get(&self, widget_id: impl Into<WidgetId>) -> Option<WidgetFocusInfo> {
        self.tree
            .get(widget_id)
            .and_then(|i| i.into_focusable(self.focus_disabled_widgets(), self.focus_hidden_widgets()))
    }

    /// Reference to the first focusable widget or parent in the tree.
    pub fn get_or_parent(&self, path: &WidgetPath) -> Option<WidgetFocusInfo> {
        self.get(path.widget_id())
            .or_else(|| path.ancestors().iter().rev().find_map(|&id| self.get(id)))
    }

    /// If the tree info contains the widget and it is focusable.
    pub fn contains(&self, widget_id: impl Into<WidgetId>) -> bool {
        self.get(widget_id).is_some()
    }
}

/// [`WidgetInfo`] extensions that build a [`WidgetFocusInfo`].
///
/// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
pub trait WidgetInfoFocusExt {
    /// Wraps the [`WidgetInfo`] in a [`WidgetFocusInfo`] even if it is not focusable.
    ///
    /// See the [`FOCUS.focus_disabled_widgets`] and [`FOCUS.focus_hidden_widgets`] config for more details on the parameters.
    ///
    /// [`FOCUS.focus_disabled_widgets`]: crate::focus::FOCUS::focus_disabled_widgets
    /// [`FOCUS.focus_hidden_widgets`]: crate::focus::FOCUS::focus_hidden_widgets
    /// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
    fn into_focus_info(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> WidgetFocusInfo;
    /// Returns a wrapped [`WidgetFocusInfo`] if the [`WidgetInfo`] is focusable.
    ///
    /// See the [`FOCUS.focus_disabled_widgets`] and [`FOCUS.focus_hidden_widgets`] config for more details on the parameters.
    ///
    /// [`FOCUS.focus_disabled_widgets`]: crate::focus::FOCUS::focus_disabled_widgets
    /// [`FOCUS.focus_hidden_widgets`]: crate::focus::FOCUS::focus_hidden_widgets
    /// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
    fn into_focusable(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> Option<WidgetFocusInfo>;
}
impl WidgetInfoFocusExt for WidgetInfo {
    fn into_focus_info(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> WidgetFocusInfo {
        WidgetFocusInfo::new(self, focus_disabled_widgets, focus_hidden_widgets)
    }
    fn into_focusable(self, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> Option<WidgetFocusInfo> {
        let r = self.into_focus_info(focus_disabled_widgets, focus_hidden_widgets);
        if r.is_focusable() { Some(r) } else { None }
    }
}

/// [`WidgetInfo`] wrapper that adds focus information for each widget.
///
/// [`WidgetInfo`]: zng_app::widget::info::WidgetInfo
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct WidgetFocusInfo {
    info: WidgetInfo,
    mode: FocusMode,
}
impl WidgetFocusInfo {
    /// Wrap a `widget_info` reference to enable focus info querying.
    ///
    /// See the [`FOCUS.focus_disabled_widgets`] and [`FOCUS.focus_hidden_widgets`] config for more details on the parameters.
    ///
    /// [`FOCUS.focus_disabled_widgets`]: crate::focus::FOCUS::focus_disabled_widgets
    /// [`FOCUS.focus_hidden_widgets`]: crate::focus::FOCUS::focus_hidden_widgets
    pub fn new(widget_info: WidgetInfo, focus_disabled_widgets: bool, focus_hidden_widgets: bool) -> Self {
        WidgetFocusInfo {
            info: widget_info,
            mode: FocusMode::new(focus_disabled_widgets, focus_hidden_widgets),
        }
    }

    /// Full widget info.
    pub fn info(&self) -> &WidgetInfo {
        &self.info
    }

    /// If [`DISABLED`] widgets are focusable in this tree.
    ///
    /// See the [`FOCUS.focus_disabled_widgets`] config for more details.
    ///
    /// [`DISABLED`]: zng_app::widget::info::Interactivity::DISABLED
    /// [`FOCUS.focus_disabled_widgets`]: crate::focus::FOCUS::focus_disabled_widgets
    pub fn focus_disabled_widgets(&self) -> bool {
        self.mode.contains(FocusMode::DISABLED)
    }

    /// If [`Hidden`] widgets are focusable in this tree.
    ///
    /// See the [`FOCUS.focus_hidden_widgets`] config for more details.
    ///
    /// [`Hidden`]: Visibility::Hidden
    /// [`FOCUS.focus_hidden_widgets`]: crate::focus::FOCUS::focus_hidden_widgets
    pub fn focus_hidden_widgets(&self) -> bool {
        self.mode.contains(FocusMode::HIDDEN)
    }

    /// Root focusable.
    pub fn root(&self) -> Self {
        self.ancestors().last().unwrap_or_else(|| self.clone())
    }

    /// Clone a reference to the [`FocusInfoTree`] that owns this widget.
    pub fn focus_tree(&self) -> FocusInfoTree {
        FocusInfoTree {
            tree: self.info.tree().clone(),
            mode: self.mode,
        }
    }

    /// If the widget is focusable.
    ///
    /// ## Note
    ///
    /// This is probably `true`, the only way to get a [`WidgetFocusInfo`] for a non-focusable widget is by
    /// calling [`into_focus_info`](WidgetInfoFocusExt::into_focus_info) or explicitly constructing one.
    ///
    /// Focus scopes are also focusable.
    pub fn is_focusable(&self) -> bool {
        self.focus_info().is_focusable()
    }

    /// Is focus scope.
    pub fn is_scope(&self) -> bool {
        self.focus_info().is_scope()
    }

    /// Is ALT focus scope.
    pub fn is_alt_scope(&self) -> bool {
        self.focus_info().is_alt_scope()
    }

    /// Gets the nested window ID, if this widget hosts a nested window.
    ///
    /// Nested window hosts always focus the nested window on focus.
    pub fn nested_window(&self) -> Option<WindowId> {
        self.info.nested_window()
    }

    /// Gets the nested window focus tree, if this widget hosts a nested window.
    pub fn nested_window_tree(&self) -> Option<FocusInfoTree> {
        self.info
            .nested_window_tree()
            .map(|t| FocusInfoTree::new(t, self.focus_disabled_widgets(), self.focus_hidden_widgets()))
    }

    fn mode_allows_focus(&self) -> bool {
        let int = self.info.interactivity();
        if self.mode.contains(FocusMode::DISABLED) {
            if int.is_blocked() {
                return false;
            }
        } else if !int.is_enabled() {
            return false;
        }

        let vis = self.info.visibility();
        if self.mode.contains(FocusMode::HIDDEN) {
            if vis == Visibility::Collapsed {
                return false;
            }
        } else if vis != Visibility::Visible {
            return false;
        }

        true
    }

    fn mode_allows_focus_ignore_blocked(&self) -> bool {
        let int = self.info.interactivity();
        if !self.mode.contains(FocusMode::DISABLED) && int.is_vis_disabled() {
            return false;
        }

        let vis = self.info.visibility();
        if self.mode.contains(FocusMode::HIDDEN) {
            if vis == Visibility::Collapsed {
                return false;
            }
        } else if vis != Visibility::Visible {
            return false;
        }

        true
    }

    /// Widget focus metadata.
    pub fn focus_info(&self) -> FocusInfo {
        if self.mode_allows_focus() {
            if let Some(builder) = self.info.meta().get(*FOCUS_INFO_ID) {
                return builder.build();
            } else if self.info.nested_window().is_some() {
                // service will actually focus nested window
                return FocusInfo::FocusScope {
                    tab_index: TabIndex::AUTO,
                    skip_directional: false,
                    tab_nav: TabNav::Contained,
                    directional_nav: DirectionalNav::Contained,
                    on_focus: FocusScopeOnFocus::FirstDescendant,
                    alt: false,
                };
            }
        }
        FocusInfo::NotFocusable
    }

    /// Widget focus metadata, all things equal except the widget interactivity is blocked.
    pub fn focus_info_ignore_blocked(&self) -> FocusInfo {
        if self.mode_allows_focus_ignore_blocked() {
            if let Some(builder) = self.info.meta().get(*FOCUS_INFO_ID) {
                return builder.build();
            }
        }
        FocusInfo::NotFocusable
    }

    /// Iterator over focusable parent -> grandparent -> .. -> root.
    pub fn ancestors(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        let focus_disabled_widgets = self.focus_disabled_widgets();
        let focus_hidden_widgets = self.focus_hidden_widgets();
        self.info.ancestors().focusable(focus_disabled_widgets, focus_hidden_widgets)
    }

    /// Iterator over self -> focusable parent -> grandparent -> .. -> root.
    pub fn self_and_ancestors(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        [self.clone()].into_iter().chain(self.ancestors())
    }

    /// Iterator over focus scopes parent -> grandparent -> .. -> root.
    pub fn scopes(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        let focus_disabled_widgets = self.focus_disabled_widgets();
        let focus_hidden_widgets = self.focus_hidden_widgets();
        self.info.ancestors().filter_map(move |i| {
            let i = i.into_focus_info(focus_disabled_widgets, focus_hidden_widgets);
            if i.is_scope() { Some(i) } else { None }
        })
    }

    /// Reference to the focusable parent that contains this widget.
    pub fn parent(&self) -> Option<WidgetFocusInfo> {
        self.ancestors().next()
    }

    /// Reference the focus scope parent that contains the widget.
    pub fn scope(&self) -> Option<WidgetFocusInfo> {
        self.scopes().next()
    }

    /// Reference the ALT focus scope *closest* with the current widget.
    ///
    /// # Closest Alt Scope
    ///
    /// - If `self` is already an ALT scope or is in one, moves to a sibling ALT scope, nested ALT scopes are ignored.
    /// - If `self` is a normal scope, moves to the first descendant ALT scope, otherwise..
    /// - Recursively searches for an ALT scope sibling up the scope tree.
    pub fn alt_scope(&self) -> Option<WidgetFocusInfo> {
        if self.in_alt_scope() {
            // We do not allow nested alt scopes, search for sibling focus scope.
            let mut alt_scope = self.clone();
            for scope in self.scopes() {
                if scope.is_alt_scope() {
                    alt_scope = scope;
                } else {
                    return scope.inner_alt_scope_skip(&alt_scope);
                }
            }
            None
        } else if self.is_scope() {
            // if we are a normal scope, try for an inner ALT scope descendant first.
            let r = self.inner_alt_scope();
            if r.is_some() {
                return r;
            }
            if let Some(scope) = self.scope() {
                // search sibling ALT scope.
                return scope.inner_alt_scope_skip(self);
            }
            None
        } else if let Some(scope) = self.scope() {
            // search sibling ALT scope.
            if self.is_focusable() {
                scope.inner_alt_scope_skip(self)
            } else {
                scope.inner_alt_scope()
            }
        } else {
            // we reached root, no ALT found.
            None
        }
    }
    fn inner_alt_scope(&self) -> Option<WidgetFocusInfo> {
        let inner_alt = self.info.meta().get(*FOCUS_INFO_ID)?.inner_alt.load(Relaxed);
        if let Some(id) = inner_alt {
            if let Some(wgt) = self.info.tree().get(id) {
                let wgt = wgt.into_focus_info(self.focus_disabled_widgets(), self.focus_hidden_widgets());
                if wgt.is_alt_scope() && wgt.info.is_descendant(&self.info) {
                    return Some(wgt);
                }
            }
        }
        None
    }
    fn inner_alt_scope_skip(self, skip: &WidgetFocusInfo) -> Option<WidgetFocusInfo> {
        if let Some(alt) = self.inner_alt_scope() {
            if !alt.info.is_descendant(&skip.info) && alt.info != skip.info {
                return Some(alt);
            }
        }
        None
    }

    /// Widget is in a ALT scope or is an ALT scope.
    pub fn in_alt_scope(&self) -> bool {
        self.is_alt_scope() || self.scopes().any(|s| s.is_alt_scope())
    }

    /// Widget the focus needs to move to when `self` gets focused.
    ///
    /// # Input
    ///
    /// * `last_focused`: A function that returns the last focused widget within a focus scope identified by `WidgetId`.
    /// * `is_tab_cycle_reentry`: If the focus returned to `self` immediately after leaving because the parent scope is `TabNav::Cycle`.
    /// * `reverse`: If the focus *reversed* into `self`.
    ///
    /// # Returns
    ///
    /// Returns the different widget the focus must move to after focusing in `self` that is a focus scope.
    ///
    /// If `self` is not a [`FocusScope`](FocusInfo::FocusScope) always returns `None`.
    pub fn on_focus_scope_move<'f>(
        &self,
        last_focused: impl FnOnce(WidgetId) -> Option<&'f WidgetPath>,
        is_tab_cycle_reentry: bool,
        reverse: bool,
    ) -> Option<WidgetFocusInfo> {
        match self.focus_info() {
            FocusInfo::FocusScope { on_focus, .. } => {
                let candidate = match on_focus {
                    FocusScopeOnFocus::FirstDescendant | FocusScopeOnFocus::FirstDescendantIgnoreBounds => {
                        if reverse {
                            self.last_tab_descendant()
                        } else {
                            self.first_tab_descendant()
                        }
                    }
                    FocusScopeOnFocus::LastFocused | FocusScopeOnFocus::LastFocusedIgnoreBounds => {
                        if is_tab_cycle_reentry { None } else { last_focused(self.info.id()) }
                            .and_then(|path| self.info.tree().get(path.widget_id()))
                            .and_then(|w| w.into_focusable(self.focus_disabled_widgets(), self.focus_hidden_widgets()))
                            .and_then(|f| {
                                if f.info.is_descendant(&self.info) {
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
                            })
                    } // fallback
                    FocusScopeOnFocus::Widget => None,
                };

                // if not IgnoreBounds and some candidate
                if let (FocusScopeOnFocus::FirstDescendant | FocusScopeOnFocus::LastFocused, Some(candidate)) = (on_focus, &candidate) {
                    if !self.info.inner_bounds().contains_rect(&candidate.info().inner_bounds()) {
                        // not fully in bounds.
                        return None;
                    }
                }

                candidate
            }
            FocusInfo::NotFocusable | FocusInfo::Focusable { .. } => None,
        }
    }

    /// Iterator over the focusable widgets contained by this widget.
    pub fn descendants(&self) -> super::iter::FocusTreeIter<w_iter::TreeIter> {
        super::iter::FocusTreeIter::new(self.info.descendants(), self.mode)
    }

    /// Iterator over self and the focusable widgets contained by it.
    pub fn self_and_descendants(&self) -> super::iter::FocusTreeIter<w_iter::TreeIter> {
        super::iter::FocusTreeIter::new(self.info.self_and_descendants(), self.mode)
    }

    /// If the focusable has any focusable descendant that is not [`TabIndex::SKIP`]
    pub fn has_tab_descendant(&self) -> bool {
        self.descendants().tree_find(Self::filter_tab_skip).is_some()
    }

    /// First descendant considering TAB index.
    pub fn first_tab_descendant(&self) -> Option<WidgetFocusInfo> {
        let mut best = (TabIndex::SKIP, self.clone());

        for d in self.descendants().tree_filter(Self::filter_tab_skip) {
            let idx = d.focus_info().tab_index();

            if idx < best.0 {
                best = (idx, d);
            }
        }

        if best.0.is_skip() { None } else { Some(best.1) }
    }

    /// Last descendant considering TAB index.
    pub fn last_tab_descendant(&self) -> Option<WidgetFocusInfo> {
        let mut best = (-1i64, self.clone());

        for d in self.descendants().tree_rev().tree_filter(Self::filter_tab_skip) {
            let idx = d.focus_info().tab_index().0 as i64;

            if idx > best.0 {
                best = (idx, d);
            }
        }

        if best.0 < 0 { None } else { Some(best.1) }
    }

    /// Iterator over all focusable widgets in the same scope after this widget.
    pub fn next_focusables(&self) -> super::iter::FocusTreeIter<w_iter::TreeIter> {
        if let Some(scope) = self.scope() {
            super::iter::FocusTreeIter::new(self.info.next_siblings_in(&scope.info), self.mode)
        } else {
            // empty
            super::iter::FocusTreeIter::new(self.info.next_siblings_in(&self.info), self.mode)
        }
    }

    /// Next focusable in the same scope after this widget.
    pub fn next_focusable(&self) -> Option<WidgetFocusInfo> {
        self.next_focusables().next()
    }

    fn filter_tab_skip(w: &WidgetFocusInfo) -> TreeFilter {
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
    pub fn next_tab_focusable(&self, skip_self: bool) -> Option<WidgetFocusInfo> {
        self.next_tab_focusable_impl(skip_self, false)
    }
    fn next_tab_focusable_impl(&self, skip_self: bool, any: bool) -> Option<WidgetFocusInfo> {
        let self_index = self.focus_info().tab_index();

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes to next in widget tree.
            return self.next_focusables().tree_find(Self::filter_tab_skip);
        }

        let mut best = (TabIndex::SKIP, self.clone());

        if !skip_self {
            for d in self.descendants().tree_filter(Self::filter_tab_skip) {
                let idx = d.focus_info().tab_index();

                if idx == self_index {
                    return Some(d);
                } else if idx < best.0 && idx > self_index {
                    if any {
                        return Some(d);
                    }
                    best = (idx, d);
                }
            }
        }

        for s in self.next_focusables().tree_filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index();

            if idx == self_index {
                return Some(s);
            } else if idx < best.0 && idx > self_index {
                if any {
                    return Some(s);
                }
                best = (idx, s);
            }
        }

        for s in self.prev_focusables().tree_filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index();

            if idx <= best.0 && idx > self_index {
                if any {
                    return Some(s);
                }
                best = (idx, s);
            }
        }

        if best.0.is_skip() { None } else { Some(best.1) }
    }

    /// Iterator over all focusable widgets in the same scope before this widget in reverse.
    pub fn prev_focusables(&self) -> super::iter::FocusTreeIter<w_iter::RevTreeIter> {
        if let Some(scope) = self.scope() {
            super::iter::FocusTreeIter::new(self.info.prev_siblings_in(&scope.info), self.mode)
        } else {
            // empty
            super::iter::FocusTreeIter::new(self.info.prev_siblings_in(&self.info), self.mode)
        }
    }

    /// Previous focusable in the same scope before this widget.
    pub fn prev_focusable(&self) -> Option<WidgetFocusInfo> {
        self.prev_focusables().next()
    }

    /// Previous focusable in the same scope after this widget respecting the TAB index.
    ///
    /// If `self` is set to [`TabIndex::SKIP`] returns the previous non-skip focusable in the same scope before this widget.
    ///
    /// If `skip_self` is `true`, does not include widgets inside `self`.
    pub fn prev_tab_focusable(&self, skip_self: bool) -> Option<WidgetFocusInfo> {
        self.prev_tab_focusable_impl(skip_self, false)
    }
    fn prev_tab_focusable_impl(&self, skip_self: bool, any: bool) -> Option<WidgetFocusInfo> {
        let self_index = self.focus_info().tab_index();

        if self_index == TabIndex::SKIP {
            // TAB from skip, goes to prev in widget tree.
            return self.prev_focusables().tree_find(Self::filter_tab_skip);
        }

        let self_index = self_index.0 as i64;
        let mut best = (-1i64, self.clone());

        if !skip_self {
            for d in self.descendants().tree_rev().tree_filter(Self::filter_tab_skip) {
                let idx = d.focus_info().tab_index().0 as i64;

                if idx == self_index {
                    return Some(d);
                } else if idx > best.0 && idx < self_index {
                    if any {
                        return Some(d);
                    }
                    best = (idx, d);
                }
            }
        }

        for s in self.prev_focusables().tree_filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index().0 as i64;

            if idx == self_index {
                return Some(s);
            } else if idx > best.0 && idx < self_index {
                if any {
                    return Some(s);
                }
                best = (idx, s);
            }
        }

        for s in self.next_focusables().tree_filter(Self::filter_tab_skip) {
            let idx = s.focus_info().tab_index().0 as i64;

            if idx >= best.0 && idx < self_index {
                if any {
                    return Some(s);
                }
                best = (idx, s);
            }
        }

        if best.0 < 0 { None } else { Some(best.1) }
    }

    /// Widget to focus when pressing TAB from this widget.
    ///
    /// Set `skip_self` to not enter `self`, that is, the focus goes to the next sibling or next sibling descendant.
    ///
    /// Returns `None` if the focus does not move to another widget.
    pub fn next_tab(&self, skip_self: bool) -> Option<WidgetFocusInfo> {
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
    pub fn prev_tab(&self, skip_self: bool) -> Option<WidgetFocusInfo> {
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

    /// Find the focusable descendant with center point nearest of `origin` within the `max_radius`.
    pub fn nearest(&self, origin: PxPoint, max_radius: Px) -> Option<WidgetFocusInfo> {
        let cast = |w: WidgetInfo| w.into_focus_info(self.focus_disabled_widgets(), self.focus_hidden_widgets());
        self.info
            .nearest_filtered(origin, max_radius, |w| cast(w.clone()).is_focusable())
            .map(cast)
    }

    /// Find the descendant with center point nearest of `origin` within the `max_radius` and approved by the `filter` closure.
    pub fn nearest_filtered(
        &self,
        origin: PxPoint,
        max_radius: Px,
        mut filter: impl FnMut(WidgetFocusInfo) -> bool,
    ) -> Option<WidgetFocusInfo> {
        let cast = |w: WidgetInfo| w.into_focus_info(self.focus_disabled_widgets(), self.focus_hidden_widgets());
        self.info
            .nearest_filtered(origin, max_radius, |w| {
                let w = cast(w.clone());
                w.is_focusable() && filter(w)
            })
            .map(cast)
    }

    /// Find the descendant with center point nearest of `origin` within the `max_radius` and inside `bounds`; and approved by the `filter` closure.
    pub fn nearest_bounded_filtered(
        &self,
        origin: PxPoint,
        max_radius: Px,
        bounds: PxRect,
        mut filter: impl FnMut(WidgetFocusInfo) -> bool,
    ) -> Option<WidgetFocusInfo> {
        let cast = |w: WidgetInfo| w.into_focus_info(self.focus_disabled_widgets(), self.focus_hidden_widgets());
        self.info
            .nearest_bounded_filtered(origin, max_radius, bounds, move |w| {
                let w = cast(w.clone());
                w.is_focusable() && filter(w)
            })
            .map(cast)
    }

    /// Find the focusable descendant with center point nearest of `origin` within the `max_distance` and with `orientation` to origin.
    pub fn nearest_oriented(&self, origin: PxPoint, max_distance: Px, orientation: Orientation2D) -> Option<WidgetFocusInfo> {
        let cast = |w: WidgetInfo| w.into_focus_info(self.focus_disabled_widgets(), self.focus_hidden_widgets());
        self.info
            .nearest_oriented_filtered(origin, max_distance, orientation, |w| cast(w.clone()).is_focusable())
            .map(cast)
    }

    /// Find the focusable descendant with center point nearest of `origin` within the `max_distance` and with `orientation`
    /// to origin that passes the `filter`.
    pub fn nearest_oriented_filtered(
        &self,
        origin: PxPoint,
        max_distance: Px,
        orientation: Orientation2D,
        mut filter: impl FnMut(WidgetFocusInfo) -> bool,
    ) -> Option<WidgetFocusInfo> {
        let cast = |w: WidgetInfo| w.into_focus_info(self.focus_disabled_widgets(), self.focus_hidden_widgets());
        self.info
            .nearest_oriented_filtered(origin, max_distance, orientation, |w| {
                let w = cast(w.clone());
                w.is_focusable() && filter(w)
            })
            .map(cast)
    }

    fn directional_from(
        &self,
        scope: &WidgetFocusInfo,
        origin: PxBox,
        orientation: Orientation2D,
        skip_self: bool,
        any: bool,
    ) -> Option<WidgetFocusInfo> {
        let self_id = self.info.id();
        let scope_id = scope.info.id();

        // don't return focus to parent from non-focusable child.
        let skip_parent = if self.is_focusable() {
            None
        } else {
            self.ancestors().next().map(|w| w.info.id())
        };

        let filter = |w: &WidgetFocusInfo| {
            let mut up_to_scope = w.self_and_ancestors().take_while(|w| w.info.id() != scope_id);

            if skip_self {
                up_to_scope.all(|w| w.info.id() != self_id && !w.focus_info().skip_directional())
            } else {
                up_to_scope.all(|w| !w.focus_info().skip_directional())
            }
        };

        let origin_center = origin.center();

        let mut oriented = scope
            .info
            .oriented(origin_center, Px::MAX, orientation)
            .chain(
                // nearby boxes (not overlapped)
                scope
                    .info
                    .oriented_box(origin, origin.width().max(origin.height()) * Px(2), orientation)
                    .filter(|w| !w.inner_bounds().to_box2d().intersects(&origin)),
            )
            .focusable(self.focus_disabled_widgets(), self.focus_hidden_widgets())
            .filter(|w| w.info.id() != scope_id && Some(w.info.id()) != skip_parent);

        if any {
            return oriented.find(filter);
        }

        let parent_range = self.parent().map(|w| w.info.descendants_range()).unwrap_or_default();

        let mut ancestor_dist = DistanceKey::NONE_MAX;
        let mut ancestor = None;
        let mut sibling_dist = DistanceKey::NONE_MAX;
        let mut sibling = None;
        let mut other_dist = DistanceKey::NONE_MAX;
        let mut other = None;

        for w in oriented {
            if filter(&w) {
                let dist = w.info.distance_key(origin_center);

                let mut is_ancestor = None;
                let mut is_ancestor = || *is_ancestor.get_or_insert_with(|| w.info.is_ancestor(&self.info));

                let mut is_sibling = None;
                let mut is_sibling = || *is_sibling.get_or_insert_with(|| parent_range.contains(&w.info));

                if dist <= ancestor_dist && is_ancestor() {
                    ancestor_dist = dist;
                    ancestor = Some(w);
                } else if dist <= sibling_dist && is_sibling() {
                    sibling_dist = dist;
                    sibling = Some(w);
                } else if dist <= other_dist && !is_ancestor() && !is_sibling() {
                    other_dist = dist;
                    other = Some(w);
                }
            }
        }

        if other_dist <= ancestor_dist && other_dist <= sibling_dist {
            other
        } else {
            sibling.or(ancestor)
        }
    }

    fn directional_next(&self, orientation: Orientation2D) -> Option<WidgetFocusInfo> {
        self.directional_next_from(orientation, self.info.inner_bounds().to_box2d())
    }

    fn directional_next_from(&self, orientation: Orientation2D, from: PxBox) -> Option<WidgetFocusInfo> {
        self.scope()
            .and_then(|s| self.directional_from(&s, from, orientation, false, false))
    }

    /// Closest focusable in the same scope above this widget.
    pub fn focusable_up(&self) -> Option<WidgetFocusInfo> {
        self.directional_next(Orientation2D::Above)
    }

    /// Closest focusable in the same scope below this widget.
    pub fn focusable_down(&self) -> Option<WidgetFocusInfo> {
        self.directional_next(Orientation2D::Below)
    }

    /// Closest focusable in the same scope to the left of this widget.
    pub fn focusable_left(&self) -> Option<WidgetFocusInfo> {
        self.directional_next(Orientation2D::Left)
    }

    /// Closest focusable in the same scope to the right of this widget.
    pub fn focusable_right(&self) -> Option<WidgetFocusInfo> {
        self.directional_next(Orientation2D::Right)
    }

    /// Widget to focus when pressing the arrow up key from this widget.
    pub fn next_up(&self) -> Option<WidgetFocusInfo> {
        let _span = tracing::trace_span!("next_up").entered();
        self.next_up_from(self.info.inner_bounds().to_box2d())
    }
    fn next_up_from(&self, origin: PxBox) -> Option<WidgetFocusInfo> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.directional_next_from(Orientation2D::Above, origin).or_else(|| {
                    let mut from = scope.info.inner_bounds();
                    from.origin.y -= Px(1);
                    from.size.height = Px(1);
                    scope.next_up_from(from.to_box2d())
                }),
                DirectionalNav::Contained => self.directional_next_from(Orientation2D::Above, origin),
                DirectionalNav::Cycle => {
                    self.directional_next_from(Orientation2D::Above, origin).or_else(|| {
                        // next up from the same X but from the bottom segment of scope spatial bounds.
                        let mut from_pt = origin.center();
                        from_pt.y = scope.info.spatial_bounds().max.y;
                        self.directional_from(
                            &scope,
                            PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                            Orientation2D::Above,
                            false,
                            false,
                        )
                    })
                }
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow right key from this widget.
    pub fn next_right(&self) -> Option<WidgetFocusInfo> {
        let _span = tracing::trace_span!("next_right").entered();
        self.next_right_from(self.info.inner_bounds().to_box2d())
    }
    fn next_right_from(&self, origin: PxBox) -> Option<WidgetFocusInfo> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.directional_next_from(Orientation2D::Right, origin).or_else(|| {
                    let mut from = scope.info.inner_bounds();
                    from.origin.x += from.size.width + Px(1);
                    from.size.width = Px(1);
                    scope.next_right_from(from.to_box2d())
                }),
                DirectionalNav::Contained => self.directional_next_from(Orientation2D::Right, origin),
                DirectionalNav::Cycle => self.directional_next_from(Orientation2D::Right, origin).or_else(|| {
                    // next right from the same Y but from the left segment of scope spatial bounds.
                    let mut from_pt = origin.center();
                    from_pt.x = scope.info.spatial_bounds().min.x;
                    self.directional_from(
                        &scope,
                        PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                        Orientation2D::Right,
                        false,
                        false,
                    )
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow down key from this widget.
    pub fn next_down(&self) -> Option<WidgetFocusInfo> {
        let _span = tracing::trace_span!("next_down").entered();
        self.next_down_from(self.info.inner_bounds().to_box2d())
    }
    fn next_down_from(&self, origin: PxBox) -> Option<WidgetFocusInfo> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.directional_next_from(Orientation2D::Below, origin).or_else(|| {
                    let mut from = scope.info.inner_bounds();
                    from.origin.y += from.size.height + Px(1);
                    from.size.height = Px(1);
                    scope.next_down_from(from.to_box2d())
                }),
                DirectionalNav::Contained => self.directional_next_from(Orientation2D::Below, origin),
                DirectionalNav::Cycle => self.directional_next_from(Orientation2D::Below, origin).or_else(|| {
                    // next down from the same X but from the top segment of scope spatial bounds.
                    let mut from_pt = origin.center();
                    from_pt.y = scope.info.spatial_bounds().min.y;
                    self.directional_from(
                        &scope,
                        PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                        Orientation2D::Below,
                        false,
                        false,
                    )
                }),
            }
        } else {
            None
        }
    }

    /// Widget to focus when pressing the arrow left key from this widget.
    pub fn next_left(&self) -> Option<WidgetFocusInfo> {
        let _span = tracing::trace_span!("next_left").entered();
        self.next_left_from(self.info.inner_bounds().to_box2d())
    }
    fn next_left_from(&self, origin: PxBox) -> Option<WidgetFocusInfo> {
        if let Some(scope) = self.scope() {
            let scope_info = scope.focus_info();
            match scope_info.directional_nav() {
                DirectionalNav::None => None,
                DirectionalNav::Continue => self.directional_next_from(Orientation2D::Left, origin).or_else(|| {
                    let mut from = scope.info.inner_bounds();
                    from.origin.x -= Px(1);
                    from.size.width = Px(1);
                    scope.next_left_from(from.to_box2d())
                }),
                DirectionalNav::Contained => self.directional_next_from(Orientation2D::Left, origin),
                DirectionalNav::Cycle => self.directional_next_from(Orientation2D::Left, origin).or_else(|| {
                    // next left from the same Y but from the right segment of scope spatial bounds.
                    let mut from_pt = origin.center();
                    from_pt.x = scope.info.spatial_bounds().max.x;
                    self.directional_from(
                        &scope,
                        PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                        Orientation2D::Left,
                        false,
                        false,
                    )
                }),
            }
        } else {
            None
        }
    }

    fn enabled_tab_nav(
        &self,
        scope: &WidgetFocusInfo,
        scope_info: FocusInfo,
        skip_self: bool,
        already_found: FocusNavAction,
    ) -> FocusNavAction {
        match scope_info.tab_nav() {
            TabNav::None => FocusNavAction::empty(),
            tab_nav @ (TabNav::Continue | TabNav::Contained) => {
                let mut nav = already_found;

                if !nav.contains(FocusNavAction::PREV) && self.prev_tab_focusable_impl(skip_self, true).is_some() {
                    nav |= FocusNavAction::PREV;
                }
                if !nav.contains(FocusNavAction::NEXT) && self.next_tab_focusable_impl(skip_self, true).is_some() {
                    nav |= FocusNavAction::NEXT;
                }

                if !nav.contains(FocusNavAction::PREV | FocusNavAction::NEXT) && tab_nav == TabNav::Continue {
                    if let Some(p_scope) = scope.scope() {
                        nav |= scope.enabled_tab_nav(&p_scope, p_scope.focus_info(), true, nav)
                    }
                }
                nav
            }
            TabNav::Cycle => {
                if scope.descendants().tree_filter(Self::filter_tab_skip).any(|w| &w != self) {
                    FocusNavAction::PREV | FocusNavAction::NEXT
                } else {
                    FocusNavAction::empty()
                }
            }
            TabNav::Once => {
                if let Some(p_scope) = scope.scope() {
                    scope.enabled_tab_nav(&p_scope, p_scope.focus_info(), true, already_found)
                } else {
                    FocusNavAction::empty()
                }
            }
        }
    }

    fn enabled_directional_nav(
        &self,
        scope: &WidgetFocusInfo,
        scope_info: FocusInfo,
        skip_self: bool,
        already_found: FocusNavAction,
    ) -> FocusNavAction {
        let directional_nav = scope_info.directional_nav();

        if directional_nav == DirectionalNav::None {
            return FocusNavAction::empty();
        }

        let mut nav = already_found;
        let from_pt = self.info.inner_bounds().to_box2d();

        if !nav.contains(FocusNavAction::UP)
            && self
                .directional_from(scope, from_pt, Orientation2D::Above, skip_self, true)
                .is_some()
        {
            nav |= FocusNavAction::UP;
        }
        if !nav.contains(FocusNavAction::RIGHT)
            && self
                .directional_from(scope, from_pt, Orientation2D::Right, skip_self, true)
                .is_some()
        {
            nav |= FocusNavAction::RIGHT;
        }
        if !nav.contains(FocusNavAction::DOWN)
            && self
                .directional_from(scope, from_pt, Orientation2D::Below, skip_self, true)
                .is_some()
        {
            nav |= FocusNavAction::DOWN;
        }
        if !nav.contains(FocusNavAction::LEFT)
            && self
                .directional_from(scope, from_pt, Orientation2D::Left, skip_self, true)
                .is_some()
        {
            nav |= FocusNavAction::LEFT;
        }

        if !nav.contains(FocusNavAction::DIRECTIONAL) {
            match directional_nav {
                DirectionalNav::Continue => {
                    if let Some(p_scope) = scope.scope() {
                        nav |= scope.enabled_directional_nav(&p_scope, p_scope.focus_info(), true, nav);
                    }
                }
                DirectionalNav::Cycle => {
                    let scope_bounds = scope.info.inner_bounds();
                    if !nav.contains(FocusNavAction::UP) {
                        let mut from_pt = from_pt.center();
                        from_pt.y = scope_bounds.max().y;
                        if self
                            .directional_from(
                                scope,
                                PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                                Orientation2D::Above,
                                true,
                                true,
                            )
                            .is_some()
                        {
                            nav |= FocusNavAction::UP;
                        }
                    }
                    if !nav.contains(FocusNavAction::RIGHT) {
                        let mut from_pt = from_pt.center();
                        from_pt.x = scope_bounds.min().x;
                        if self
                            .directional_from(
                                scope,
                                PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                                Orientation2D::Right,
                                true,
                                true,
                            )
                            .is_some()
                        {
                            nav |= FocusNavAction::RIGHT;
                        }
                    }
                    if !nav.contains(FocusNavAction::DOWN) {
                        let mut from_pt = from_pt.center();
                        from_pt.y = scope_bounds.min().y;
                        if self
                            .directional_from(
                                scope,
                                PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                                Orientation2D::Below,
                                true,
                                true,
                            )
                            .is_some()
                        {
                            nav |= FocusNavAction::DOWN;
                        }
                    }
                    if !nav.contains(FocusNavAction::LEFT) {
                        let mut from_pt = from_pt.center();
                        from_pt.x = scope_bounds.max().x;
                        if self
                            .directional_from(
                                scope,
                                PxRect::new(from_pt, PxSize::splat(Px(1))).to_box2d(),
                                Orientation2D::Left,
                                true,
                                true,
                            )
                            .is_some()
                        {
                            nav |= FocusNavAction::LEFT;
                        }
                    }

                    if !nav.contains(FocusNavAction::DIRECTIONAL) {
                        let info = self.focus_info();

                        if info.is_scope() && matches!(info.directional_nav(), DirectionalNav::Continue) {
                            // continue scope as single child of cycle scope.
                            if nav.contains(FocusNavAction::UP) || nav.contains(FocusNavAction::DOWN) {
                                nav |= FocusNavAction::UP | FocusNavAction::DOWN;
                            }
                            if nav.contains(FocusNavAction::LEFT) || nav.contains(FocusNavAction::RIGHT) {
                                nav |= FocusNavAction::LEFT | FocusNavAction::RIGHT;
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        nav
    }

    /// Focus navigation actions that can move the focus away from this item.
    pub fn enabled_nav(&self) -> FocusNavAction {
        let _span = tracing::trace_span!("enabled_nav").entered();

        let mut nav = FocusNavAction::empty();

        if let Some(scope) = self.scope() {
            nav |= FocusNavAction::EXIT;
            nav.set(FocusNavAction::ENTER, self.descendants().next().is_some());

            let scope_info = scope.focus_info();

            nav |= self.enabled_tab_nav(&scope, scope_info, false, FocusNavAction::empty());
            nav |= self.enabled_directional_nav(&scope, scope_info, false, FocusNavAction::empty());
        }

        nav.set(FocusNavAction::ALT, self.in_alt_scope() || self.alt_scope().is_some());

        nav
    }
}

/// Focus metadata associated with a widget info tree.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
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
#[derive(Clone, Copy, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum FocusScopeOnFocus {
    /// Just focus the scope widget.
    Widget,
    /// Focus the first descendant considering the TAB index, if the scope has no descendants
    /// behaves like [`Widget`].
    ///
    /// Focus the last descendant if the focus is *reversing* in, e.g. in a SHIFT+TAB action.
    ///
    /// Behaves like [`Widget`] if the first(or last) descendant inner-bounds is not fully contained
    /// by the scope inner-bounds.
    ///
    /// [`Widget`]: Self::Widget
    FirstDescendant,
    /// Focus the descendant that was last focused before focus moved out of the scope. If the
    /// scope cannot return focus, behaves like [`FirstDescendant`].
    ///
    /// If the scope is the only child of a parent that is `TabNav::Cycle` and the focus just exited and
    /// returned in a cycle action, behaves like [`FirstDescendant`].
    ///
    /// Behaves like [`Widget`] if the first(or last) descendant inner-bounds is not fully contained
    /// by the scope inner-bounds.
    ///
    /// [`Widget`]: Self::Widget
    /// [`FirstDescendant`]: Self::FirstDescendant
    LastFocused,

    /// Like [`FirstDescendant`], but also focus the descendant even if it's inner-bounds
    /// is not fully contained by the scope inner-bounds.
    ///
    /// The expectation is that the descendant is already visible or will be made visible when
    /// it receives focus, a scroll scope will scroll to make the descendant visible for example.
    ///
    /// [`FirstDescendant`]: Self::FirstDescendant
    FirstDescendantIgnoreBounds,

    /// Like [`LastFocused`], but also focus the descendant even if it's inner-bounds
    /// is not fully contained by the scope inner-bounds.
    ///
    /// The expectation is that the descendant is already visible or will be made visible when
    /// it receives focus, a scroll scope will scroll to make the descendant visible for example.
    ///
    /// [`LastFocused`]: Self::LastFocused
    LastFocusedIgnoreBounds,
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
            FocusScopeOnFocus::FirstDescendantIgnoreBounds => write!(f, "FirstDescendantIgnoreBounds"),
            FocusScopeOnFocus::LastFocusedIgnoreBounds => write!(f, "LastFocusedIgnoreBounds"),
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

static_id! {
    static ref FOCUS_INFO_ID: StateId<FocusInfoData>;
    static ref FOCUS_TREE_ID: StateId<FocusTreeData>;
}

#[derive(Default)]
pub(super) struct FocusTreeData {
    alt_scopes: Mutex<IdSet<WidgetId>>,
}
impl FocusTreeData {
    pub(super) fn consolidate_alt_scopes(prev_tree: &WidgetInfoTree, new_tree: &WidgetInfoTree) {
        // reused widgets don't insert build-meta, so we add the previous ALT scopes and validate everything.

        let prev = prev_tree
            .build_meta()
            .get(*FOCUS_TREE_ID)
            .map(|d| d.alt_scopes.lock().clone())
            .unwrap_or_default();

        let mut alt_scopes = prev;
        if let Some(data) = new_tree.build_meta().get(*FOCUS_TREE_ID) {
            alt_scopes.extend(data.alt_scopes.lock().iter());
        }

        alt_scopes.retain(|id| {
            if let Some(wgt) = new_tree.get(*id) {
                if let Some(info) = wgt.meta().get(*FOCUS_INFO_ID) {
                    if info.build().is_alt_scope() {
                        for parent in wgt.ancestors() {
                            if let Some(info) = parent.meta().get(*FOCUS_INFO_ID) {
                                if info.build().is_scope() {
                                    info.inner_alt.store(Some(*id), Relaxed);
                                    break;
                                }
                            }
                        }

                        return true;
                    }
                }
            }
            false
        });

        if let Some(data) = new_tree.build_meta().get(*FOCUS_TREE_ID) {
            *data.alt_scopes.lock() = alt_scopes;
        }
    }
}

#[derive(Default, Debug)]
struct FocusInfoData {
    focusable: Option<bool>,
    scope: Option<bool>,
    alt_scope: bool,
    on_focus: FocusScopeOnFocus,
    tab_index: Option<TabIndex>,
    tab_nav: Option<TabNav>,
    directional_nav: Option<DirectionalNav>,
    skip_directional: Option<bool>,

    inner_alt: Atomic<Option<WidgetId>>,

    access_handler_registered: bool,
}
impl FocusInfoData {
    /// Build a [`FocusInfo`] from the collected configuration in `self`.
    ///
    /// See [`FocusInfoBuilder`] for a review of the algorithm.
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

/// Builder for [`FocusInfo`] accessible in a [`WidgetInfoBuilder`].
///
/// There are multiple focusable metadata that can be set on a widget. These rules define how the focusable
/// state of a widget is derived from the focusable metadata.
///
/// ### Rules
///
/// The widget is not focusable nor a focus scope if it set [`focusable`](Self::focusable) to `false`.
///
/// The widget is a *focus scope* if it set [`scope`](Self::scope) to `true` **or** if it set [`tab_nav`](Self::tab_nav) or
/// [`directional_nav`](Self::directional_nav) and did not set [`scope`](Self::scope) to `false`.
///
/// The widget is *focusable* if it set [`focusable`](Self::focusable) to `true` **or** if it set the [`tab_index`](Self::tab_index).
///
/// The widget is a *focus scope* if it sets [`nested_window`](NestedWindowWidgetInfoExt::nested_window), but the focus will always move inside
/// the nested window.
///
/// The widget is not focusable if it did not set any of the members mentioned.
///
/// ##### Tab Index
///
/// If the [`tab_index`](Self::tab_index) was not set but the widget is focusable or a focus scope, the [`TabIndex::AUTO`]
/// is used for the widget.
///
/// ##### Skip Directional
///
/// If the [`skip_directional`](Self::skip_directional) was not set but the widget is focusable or a focus scope, it is
/// set to `false` for the widget.
///
/// ##### Focus Scope
///
/// If the widget is a focus scope, it is configured using [`alt_scope`](Self::alt_scope) and [`on_focus`](Self::on_focus).
/// If the widget is not a scope these members are ignored.
///
/// ##### Tab Navigation
///
/// If [`tab_nav`](Self::tab_nav) is not set but the widget is a focus scope, [`TabNav::Continue`] is used.
///
/// ##### Directional Navigation
///
/// If [`directional_nav`](Self::directional_nav) is not set but the widget is a focus scope, [`DirectionalNav::Continue`] is used.
///
/// [`WidgetInfoBuilder`]: zng_app::widget::info::WidgetInfoBuilder
/// [`new`]: Self::new
pub struct FocusInfoBuilder<'a>(&'a mut WidgetInfoBuilder);
impl<'a> FocusInfoBuilder<'a> {
    /// New the builder.
    pub fn new(builder: &'a mut WidgetInfoBuilder) -> Self {
        let mut r = Self(builder);
        r.with_tree_data(|_| {}); // ensure that build meta is allocated.
        r
    }

    fn with_data<R>(&mut self, visitor: impl FnOnce(&mut FocusInfoData) -> R) -> R {
        let mut access = self.0.access().is_some();

        let r = self.0.with_meta(|m| {
            let data = m.into_entry(*FOCUS_INFO_ID).or_default();

            if access {
                access = !std::mem::replace(&mut data.access_handler_registered, true);
            }

            visitor(data)
        });

        if access {
            // access info required and not registered
            self.0.access().unwrap().on_access_build(|args| {
                if args.widget.info().clone().into_focusable(true, false).is_some() {
                    args.node.commands.push(zng_view_api::access::AccessCmdName::Focus);
                }
            });
        }

        r
    }

    fn with_tree_data<R>(&mut self, visitor: impl FnOnce(&mut FocusTreeData) -> R) -> R {
        self.0.with_build_meta(|m| visitor(m.into_entry(*FOCUS_TREE_ID).or_default()))
    }

    /// If the widget is definitely focusable or not.
    pub fn focusable(&mut self, is_focusable: bool) -> &mut Self {
        self.with_data(|data| {
            data.focusable = Some(is_focusable);
        });
        self
    }

    /// Sets [`focusable`], only if it was not already set.
    ///
    /// [`focusable`]: Self::focusable
    pub fn focusable_passive(&mut self, is_focusable: bool) -> &mut Self {
        self.with_data(|data| {
            if data.focusable.is_none() {
                data.focusable = Some(is_focusable);
            }
        });
        self
    }

    /// If the widget is definitely a focus scope or not.
    pub fn scope(&mut self, is_focus_scope: bool) -> &mut Self {
        self.with_data(|data| {
            data.scope = Some(is_focus_scope);
        });
        self
    }

    /// If the widget is definitely an ALT focus scope or not.
    ///
    /// If `true` this also sets `TabIndex::SKIP`, `skip_directional_nav`, `TabNav::Cycle` and `DirectionalNav::Cycle` as default.
    pub fn alt_scope(&mut self, is_alt_focus_scope: bool) -> &mut Self {
        self.with_data(|data| {
            data.alt_scope = is_alt_focus_scope;
            if is_alt_focus_scope {
                data.scope = Some(true);

                if data.tab_index.is_none() {
                    data.tab_index = Some(TabIndex::SKIP);
                }
                if data.tab_nav.is_none() {
                    data.tab_nav = Some(TabNav::Cycle);
                }
                if data.directional_nav.is_none() {
                    data.directional_nav = Some(DirectionalNav::Cycle);
                }
                if data.skip_directional.is_none() {
                    data.skip_directional = Some(true);
                }
            }
        });
        if is_alt_focus_scope {
            let wgt_id = self.0.widget_id();
            self.with_tree_data(|d| d.alt_scopes.lock().insert(wgt_id));
        }
        self
    }

    /// When the widget is a focus scope, its behavior on receiving direct focus.
    pub fn on_focus(&mut self, as_focus_scope_on_focus: FocusScopeOnFocus) -> &mut Self {
        self.with_data(|data| {
            data.on_focus = as_focus_scope_on_focus;
        });
        self
    }

    /// Widget TAB index.
    pub fn tab_index(&mut self, tab_index: TabIndex) -> &mut Self {
        self.with_data(|data| {
            data.tab_index = Some(tab_index);
        });
        self
    }

    /// TAB navigation within this widget, if set turns the widget into a focus scope.
    pub fn tab_nav(&mut self, scope_tab_nav: TabNav) -> &mut Self {
        self.with_data(|data| {
            data.tab_nav = Some(scope_tab_nav);
        });
        self
    }

    /// Directional navigation within this widget, if set turns the widget into a focus scope.
    pub fn directional_nav(&mut self, scope_directional_nav: DirectionalNav) -> &mut Self {
        self.with_data(|data| {
            data.directional_nav = Some(scope_directional_nav);
        });
        self
    }
    /// If directional navigation skips over this widget.
    pub fn skip_directional(&mut self, skip: bool) -> &mut Self {
        self.with_data(|data| {
            data.skip_directional = Some(skip);
        });
        self
    }
}
