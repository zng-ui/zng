//! Focus manager, events and services.

use crate::core::app::AppExtension;
use crate::core::context::*;
use crate::core::event::*;
use crate::core::keyboard::*;
use crate::core::mouse::*;
use crate::core::render::{FrameInfo, WidgetInfo, WidgetPath};
use crate::core::types::*;

event_args! {
    /// [FocusChanged] event args.
    pub struct FocusChangedArgs {
        /// Previously focused widget.
        pub prev_focus: Option<WidgetPath>,

        /// Newly focused widget.
        pub new_focus: Option<WidgetPath>,

        ..

        /// If the widget is [prev_focus](FocusChangedArgs::prev_focus) or
        /// [`new_focus`](FocusChangedArgs::new_focus).
        fn concerns_widget(&self, ctx: &mut WidgetContext) -> bool {
            if let Some(prev) = &self.prev_focus {
                if prev.widget_id() == ctx.widget_id {
                    return true
                }
            }

            if let Some(new) = &self.new_focus {
                if new.widget_id() == ctx.widget_id {
                    return true
                }
            }

            false
        }
    }
}

state_key! {
    pub(crate) struct IsFocusable: bool;
    pub(crate) struct FocusTabIndex: TabIndex;
    pub(crate) struct IsFocusScope: bool;
    pub(crate) struct FocusTabNav: TabNav;
    pub(crate) struct FocusDirectionalNav: DirectionalNav;
}

/// Widget order index during TAB navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TabIndex(u32);

impl TabIndex {
    /// Widget is skipped during TAB navigation.
    pub const SKIP: TabIndex = TabIndex(0);

    /// Widget is focused during TAB navigation using its order of declaration.
    pub const AUTO: TabIndex = TabIndex(u32::max_value());

    /// If is [`SKIP`](TabIndex::SKIP).
    #[inline]
    pub fn is_skip(self) -> bool {
        self == Self::SKIP
    }

    /// If is [`AUTO`](TabIndex::AUTO).
    #[inline]
    pub fn is_auto(self) -> bool {
        self == Self::AUTO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TabNav {
    None,
    Continue,
    Contained,
    Cycle,
    Once,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DirectionalNav {
    None,
    Continue,
    Contained,
    Cycle,
}

pub struct FocusChanged;

impl Event for FocusChanged {
    type Args = FocusChangedArgs;
}

/// Application extension that manages keyboard focus. Provides the [FocusChanged] event
/// and [Focus] service.
pub struct FocusManager {
    focused: Option<WidgetId>,
    focus_changed: EventEmitter<FocusChangedArgs>,
    mouse_down: EventListener<MouseInputArgs>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focused: None,
            focus_changed: EventEmitter::new(false),
            mouse_down: EventListener::never(false),
        }
    }
}

impl AppExtension for FocusManager {
    fn init(&mut self, ctx: &mut AppInitContext) {
        self.mouse_down = ctx.events.listen::<MouseDown>();
        ctx.services.register(Focus::new(ctx.updates.notifier().clone()))
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
        for args in self.mouse_down.updates(ctx.events) {
            //if todo!() {
            //    ctx.services.require::<Focus>().focus_widget(ctx.widget_id);
            //}
        }
        if let Some(request) = ctx.services.req::<Focus>().request.take() {
            todo!()
        }
    }
}

/// Keyboard focus service.
pub struct Focus {
    request: Option<FocusRequest>,
    update_notifier: UpdateNotifier,
    focused: Option<WidgetPath>,
}

impl Focus {
    #[inline]
    pub fn new(update_notifier: UpdateNotifier) -> Self {
        Focus {
            request: None,
            update_notifier,
            focused: None,
        }
    }

    /// Current focused widget.
    #[inline]
    pub fn focused(&self) -> Option<&WidgetPath> {
        self.focused.as_ref()
    }

    #[inline]
    pub fn focus(&mut self, request: FocusRequest) {
        self.request = Some(request);
        self.update_notifier.push_update();
    }

    #[inline]
    pub fn focus_widget(&mut self, widget_id: WidgetId) {
        self.focus(FocusRequest::Direct(widget_id))
    }

    #[inline]
    pub fn focus_next(&mut self) {
        self.focus(FocusRequest::Next);
    }

    #[inline]
    pub fn focus_prev(&mut self) {
        self.focus(FocusRequest::Prev);
    }

    #[inline]
    pub fn focus_left(&mut self) {
        self.focus(FocusRequest::Left);
    }

    #[inline]
    pub fn focus_right(&mut self) {
        self.focus(FocusRequest::Right);
    }

    #[inline]
    pub fn focus_up(&mut self) {
        self.focus(FocusRequest::Up);
    }

    #[inline]
    pub fn focus_down(&mut self) {
        self.focus(FocusRequest::Down);
    }
}

impl AppService for Focus {}

/// Focus change request.
#[derive(Clone, Copy, Debug)]
pub enum FocusRequest {
    /// Move focus to widget.
    Direct(WidgetId),

    /// Move focus to next from current in screen, or to first in screen.
    Next,
    /// Move focus to previous from current in screen, or to last in screen.
    Prev,

    /// Move focus to the left of current.
    Left,
    /// Move focus to the right of current.
    Right,
    /// Move focus above current.
    Up,
    /// Move focus bellow current.
    Down,
}

pub struct FrameFocusInfo<'a> {
    /// Full frame info.
    pub info: &'a FrameInfo,
}

impl<'a> FrameFocusInfo<'a> {
    #[inline]
    pub fn new(frame_info: &'a FrameInfo) -> Self {
        FrameFocusInfo { info: frame_info }
    }

    /// Reference to the root widget in the frame.
    ///
    /// The root is usually a focusable focus scope but it may not be. This
    /// is the only method that returns a [WidgetFocusInfo] that may not be focusable.
    #[inline]
    pub fn root(&self) -> WidgetFocusInfo {
        WidgetFocusInfo::new(self.info.root())
    }

    /// Reference to the widget in the frame, if it is present and has focus info.
    #[inline]
    pub fn find(&self, widget_id: WidgetId) -> Option<WidgetFocusInfo> {
        self.info.find(widget_id).and_then(|i| i.as_focusable())
    }

    /// If the frame info contains the widget and it is focusable.
    #[inline]
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.find(widget_id).is_some()
    }
}

/// [WidgetInfo] extensions that build a [WidgetFocusInfo].
pub trait WidgetInfoFocusExt<'a> {
    /// Wraps the [WidgetInfo] in a [WidgetFocusInfo] even if it is not focusable.
    fn as_focus_info(self) -> WidgetFocusInfo<'a>;

    /// Returns a wrapped [WidgetFocusInfo] if the [WidgetInfo] is focusable.
    fn as_focusable(self) -> Option<WidgetFocusInfo<'a>>;
}

impl<'a> WidgetInfoFocusExt<'a> for WidgetInfo<'a> {
    fn as_focus_info(self) -> WidgetFocusInfo<'a> {
        WidgetFocusInfo::new(self)
    }

    fn as_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        let r = self.as_focus_info();
        if r.is_focusable() {
            Some(r)
        } else {
            None
        }
    }
}

/// [WidgetInfo] wrapper that uses focus metadata for computing navigation.
#[derive(Clone, Copy)]
pub struct WidgetFocusInfo<'a> {
    /// Full widget info.
    pub info: WidgetInfo<'a>,
}

impl<'a> WidgetFocusInfo<'a> {
    #[inline]
    pub fn new(widget_info: WidgetInfo<'a>) -> Self {
        WidgetFocusInfo { info: widget_info }
    }

    /// Root focusable.
    #[inline]
    pub fn root(self) -> Self {
        self.ancestors().last().unwrap_or(self)
    }

    #[inline]
    pub fn is_focusable(self) -> bool {
        self.focus_info().is_focusable()
    }

    /// Is focus scope.
    #[inline]
    pub fn is_scope(self) -> bool {
        self.focus_info().is_scope()
    }

    #[inline]
    pub fn focus_info(self) -> FocusInfo {
        let m = self.info.meta();
        match (
            m.get(IsFocusable).copied(),
            m.get(IsFocusScope).copied(),
            m.get(FocusTabIndex).copied(),
            m.get(FocusTabNav).copied(),
            m.get(FocusDirectionalNav).copied(),
        ) {
            // Set as not focusable.
            (Some(false), _, _, _, _) => FocusInfo::NotFocusable,

            // Set as focus scope and not set as not focusable
            // or set tab navigation and did not set as not focus scope
            // or set directional navigation and did not set as not focus scope.
            (_, Some(true), idx, tab, dir) | (_, None, idx, tab @ Some(_), dir) | (_, None, idx, tab, dir @ Some(_)) => {
                FocusInfo::FocusScope(
                    idx.unwrap_or(TabIndex::AUTO),
                    tab.unwrap_or(TabNav::Continue),
                    dir.unwrap_or(DirectionalNav::None),
                )
            }

            // Set as focusable and was not focus scope
            // or set tab index and was not focus scope and did not set as not focusable.
            (Some(true), _, idx, _, _) | (_, _, idx @ Some(_), _, _) => FocusInfo::Focusable(idx.unwrap_or(TabIndex::AUTO)),

            _ => FocusInfo::NotFocusable,
        }
    }

    /// Iterator over focusable parent -> grant-parent -> .. -> root.
    #[inline]
    pub fn ancestors(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().focusable()
    }

    /// Iterator over focus scopes parent -> grant-parent -> .. -> root.
    #[inline]
    pub fn scopes(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.ancestors().filter_map(|i| {
            let i = i.as_focus_info();
            if i.is_scope() {
                Some(i)
            } else {
                None
            }
        })
    }

    /// Reference to the focusable parent that contains this widget.
    #[inline]
    pub fn parent(self) -> Option<WidgetFocusInfo<'a>> {
        self.ancestors().next()
    }

    /// Reference the focus scope parent that contains the widget.
    #[inline]
    pub fn scope(self) -> Option<WidgetFocusInfo<'a>> {
        let scope = self.scopes().next();
        if scope.is_some() {
            scope
        } else if self.is_scope() {
            Some(self)
        } else {
            None
        }
    }

    /// Iterator over the focusable widgets contained by this widget.
    #[inline]
    pub fn descendants(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        self.info.descendants().focusable()
    }

    /// Iterator over all focusable widgets in the same scope after this widget.
    #[inline]
    pub fn next_focusables(self) -> impl Iterator<Item = WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();
        self.scope()
            .into_iter()
            .flat_map(|s| s.descendants())
            .skip_while(move |f| f.info.widget_id() != self_id)
            .skip(1)
    }

    /// Next focusable in the same scope after this widget.
    #[inline]
    pub fn next_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        self.next_focusables().next()
    }

    /// Iterator over all focusable widgets in the same scope before this widget in reverse.
    #[inline]
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

    /// Previous focusable in the same scope after this widget.
    #[inline]
    pub fn prev_focusable(self) -> Option<WidgetFocusInfo<'a>> {
        let self_id = self.info.widget_id();

        self.scope()
            .and_then(move |s| s.descendants().take_while(move |f| f.info.widget_id() != self_id).last())
    }

    /// Widget to focus when pressing TAB from this widget.
    #[inline]
    pub fn tab(self) -> WidgetFocusInfo<'a> {
        todo!()
    }

    /// Widget to focus when pressing CTRL+TAB from this widget.
    #[inline]
    pub fn ctrl_tab(self) -> WidgetFocusInfo<'a> {
        todo!()
    }
}

/// Filter-maps an iterator of [WidgetInfo] to [WidgetFocusInfo].
pub trait IterFocusable<'a, I: Iterator<Item = WidgetInfo<'a>>> {
    fn focusable(self) -> std::iter::FilterMap<I, fn(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>>;
}

impl<'a, I: Iterator<Item = WidgetInfo<'a>>> IterFocusable<'a, I> for I {
    fn focusable(self) -> std::iter::FilterMap<I, fn(WidgetInfo<'a>) -> Option<WidgetFocusInfo<'a>>> {
        self.filter_map(|i| i.as_focusable())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FocusInfo {
    NotFocusable,
    Focusable(TabIndex),
    FocusScope(TabIndex, TabNav, DirectionalNav),
}

impl FocusInfo {
    #[inline]
    pub fn is_focusable(self) -> bool {
        match self {
            FocusInfo::NotFocusable => false,
            _ => true,
        }
    }

    #[inline]
    pub fn is_scope(self) -> bool {
        match self {
            FocusInfo::FocusScope(..) => true,
            _ => false,
        }
    }
}
