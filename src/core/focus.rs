use crate::core::app::{AppEvent, AppExtension};
use crate::core::context::*;
use crate::core::event::*;
use crate::core::frame::FrameBuilder;
use crate::core::types::*;
use crate::core::var::*;
use crate::core::UiNode;

event_args! {
    /// [FocusChanged] event args.
    pub struct FocusChangedArgs {
        pub window_id: WindowId,
        pub widget_id: WidgetId,
    }
}

pub struct FocusChanged;

impl Event for FocusChanged {
    type Args = FocusChangedArgs;
}

pub struct FocusManager {
    focused: Option<WidgetId>,
    focus_changed: EventEmitter<FocusChangedArgs>,
}

impl Default for FocusManager {
    fn default() -> Self {
        Self {
            focused: None,
            focus_changed: EventEmitter::new(false),
        }
    }
}

impl AppExtension for FocusManager {
    fn init(&mut self, ctx: &mut AppInitContext) {
        ctx.services.register(Focus::new(ctx.updates.notifier().clone()))
    }

    fn update(&mut self, update: UpdateRequest, ctx: &mut AppContext) {
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
