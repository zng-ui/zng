use crate::core::app::{AppEvent, AppExtension};
use crate::core::context::*;
use crate::core::event::*;
use crate::core::events::*;
use crate::core::frame::FrameBuilder;
use crate::core::frame::{FrameInfo, WidgetInfo};
use crate::core::types::*;
use crate::core::var::*;
use crate::core::UiNode;

event_args! {
    /// [FocusChanged] event args.
    pub struct FocusChangedArgs {
        /// Previously focused widget.
        pub prev_focus: Option<WidgetId>,

        /// Newly focused widget.
        pub new_focus: Option<WidgetId>,

        fn concerns_widget(&self, ctx: &mut WidgetContext) {
            //! If the widget is [prev_focus] or [new_focus].

            let ctx = Some(ctx.widget_id);
            self.new_focus == ctx || self.prev_focus == ctx
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

    /// If is [SKIP].
    #[inline]
    pub fn is_skip(self) -> bool {
        self == Self::SKIP
    }

    /// If is [AUTO].
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
        if let Some(request) = ctx.services.require::<Focus>().request.take() {
            todo!()
        }
    }
}

pub struct Focus {
    request: Option<FocusRequest>,
    update_notifier: UpdateNotifier,
}

impl Focus {
    #[inline]
    pub fn new(update_notifier: UpdateNotifier) -> Self {
        Focus {
            request: None,
            update_notifier,
        }
    }

    #[inline]
    pub fn focus(&mut self, request: FocusRequest) {
        self.request = Some(request);
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

impl Service for Focus {}

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
    info: &'a FrameInfo,
}

impl<'a> FrameFocusInfo<'a> {
    pub fn new(frame_info: &'a FrameInfo) -> Self {
        FrameFocusInfo { info: frame_info }
    }

    /// Reference to the root widget in the frame.
    pub fn root(&self) -> WidgetFocusInfo {
        WidgetFocusInfo::new(self.info.root())
    }
}

pub struct WidgetFocusInfo<'a> {
    pub info: WidgetInfo<'a>,
}

impl<'a> WidgetFocusInfo<'a> {
    pub fn new(widget_info: WidgetInfo<'a>) -> Self {
        WidgetFocusInfo { info: widget_info }
    }

    pub fn focus_info(&self) -> FocusInfo {
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

            // Set as focus scope and not set as not focusable.
            (_, Some(true), idx, tab, dir) => FocusInfo::FocusScope(
                idx.unwrap_or(TabIndex::AUTO),
                tab.unwrap_or(TabNav::Continue),
                dir.unwrap_or(DirectionalNav::None),
            ),

            // Set tab nav and did not set as not focus scope.
            (_, None, idx, Some(tab), dir) => {
                FocusInfo::FocusScope(idx.unwrap_or(TabIndex::AUTO), tab, dir.unwrap_or(DirectionalNav::None))
            }

            // Set directional nav and did not set as not focus scope.
            (_, None, idx, tab, Some(dir)) => {
                FocusInfo::FocusScope(idx.unwrap_or(TabIndex::AUTO), tab.unwrap_or(TabNav::Continue), dir)
            }

            // Set as focusable and was not focus scope.
            (Some(true), _, idx, _, _) => FocusInfo::Focusable(idx.unwrap_or(TabIndex::AUTO)),

            // Set tab index and was not focus scope and did not set as not focusable.
            (_, _, Some(idx), _, _) => FocusInfo::Focusable(idx),

            _ => FocusInfo::NotFocusable,
        }
    }

    /// Iterator over all next widgets within the same parent that are focusable.
    #[inline]
    pub fn next_siblings(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        self.info.next_siblings().filter_map(|n| {
            let n = WidgetFocusInfo::new(n);

            if n.focus_info().is_focusable() {
                Some(n)
            } else {
                None
            }
        })
    }

    /// Next focusable sibling.
    #[inline]
    pub fn next_sibling(&self) -> Option<WidgetFocusInfo> {
        self.next_siblings().next()
    }
}

#[derive(Debug, Clone)]
pub enum FocusInfo {
    NotFocusable,
    Focusable(TabIndex),
    FocusScope(TabIndex, TabNav, DirectionalNav),
}

impl FocusInfo {
    pub fn is_focusable(&self) -> bool {
        match self {
            FocusInfo::NotFocusable => false,
            _ => true,
        }
    }
}
