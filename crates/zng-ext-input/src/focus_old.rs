//! Keyboard focus manager.
//!
//! # Events
//!
//! Events this extension provides.
//!
//! * [`FOCUS_CHANGED_EVENT`]
//! * [`RETURN_FOCUS_CHANGED_EVENT`]
//!
//! # Services
//!
//! Services this extension provides.
//!
//! * [`FOCUS`]

use zng_app::{
    APP, DInstant, INSTANT,
    access::{ACCESS_CLICK_EVENT, ACCESS_FOCUS_EVENT, ACCESS_FOCUS_NAV_ORIGIN_EVENT},
    event::{event, event_args},
    hn,
    update::{InfoUpdates, RenderUpdates, UPDATES},
    view_process::raw_events::RAW_KEY_INPUT_EVENT,
    widget::{
        WidgetId,
        info::{InteractionPath, WIDGET_TREE_CHANGED_EVENT, WidgetBoundsInfo, WidgetInfoTree},
    },
    window::WindowId,
};

pub mod cmd;
use cmd::FocusCommands;
use zng_app_context::app_local;
use zng_ext_window::{WINDOW_FOCUS_CHANGED_EVENT, WINDOWS, WINDOWS_FOCUS};
use zng_layout::unit::{Px, PxPoint, PxRect, TimeUnits};
use zng_unique_id::{IdEntry, IdMap};
use zng_var::{Var, var};
use zng_view_api::window::FrameId;

use std::{mem, time::Duration};

use crate::{mouse::MOUSE_INPUT_EVENT, touch::TOUCH_INPUT_EVENT};

impl AppExtension for FocusManager {
    fn render(&mut self, _: &mut RenderUpdates, _: &mut RenderUpdates) {
        if let Some(tree) = self.pending_render.take() {
            self.on_info_tree_update(tree);
        } else {
            // update visibility or enabled commands, they may have changed if the `spatial_frame_id` changed.
            let focus = FOCUS_SV.read();
            let mut invalidated_cmds_or_focused = None;

            if let Some(f) = &focus.focused {
                let w_id = f.path.window_id();
                if let Ok(tree) = WINDOWS.widget_tree(w_id)
                    && focus.enabled_nav.needs_refresh(&tree)
                {
                    invalidated_cmds_or_focused = Some(tree);
                }
            }

            if let Some(tree) = invalidated_cmds_or_focused {
                drop(focus);
                self.on_info_tree_update(tree);
            }
        }
    }

    fn update(&mut self) {
        let mut focus = FOCUS_SV.write();
        if let Some((request, is_retry)) = focus.request.take_update() {
            focus.pending_highlight = false;
            let args = focus.fulfill_request(request, is_retry);
            self.notify(&mut focus, args);
        } else if mem::take(&mut focus.pending_highlight) {
            let args = focus.continue_focus_highlight(true);
            self.notify(&mut focus, args);
        }

        if let Some(wgt_id) = focus.navigation_origin_var.get_new()
            && wgt_id != focus.navigation_origin
        {
            focus.navigation_origin = wgt_id;
            focus.update_enabled_nav_with_origin();
            let commands = self.commands.as_mut().unwrap();
            commands.update_enabled(focus.enabled_nav.nav);
        }
    }

    fn info(&mut self, _: &mut InfoUpdates) {
        let mut focus = FOCUS_SV.write();
        if let Some(r) = focus.request.take_info() {
            focus.request = PendingFocusRequest::RetryUpdate(r);
            UPDATES.update(None);
        }
    }
}

app_local! {
    static FOCUS_SV: FocusService = {
        hooks();
        FocusService::new()
    };
}

fn hooks() {
    WIDGET_TREE_CHANGED_EVENT
        .hook(|args| {
            let mut s = FOCUS_SV.write();
            if s.focused.as_ref().map(|f| f.path.window_id() == args.window_id).unwrap_or_default() {
                // we need up-to-date due to visibility or spatial movement and that is affected by both layout and render.
                // so we delay responding to the event if a render or layout was requested when the tree was invalidated.
                if UPDATES.is_pending_render(args.window_id) {
                    s.pending_render = Some(args.tree.clone());
                } else {
                    // no visual change, update interactivity changes.
                    s.pending_render = None;
                    s.on_info_tree_update(args.tree.clone());
                }
            }
            if !args.is_update {
                focus_info::FocusTreeData::consolidate_alt_scopes(&args.prev_tree, &args.tree);
            }
            true
        })
        .perm();

    ACCESS_FOCUS_EVENT
        .hook(|args| {
            let is_focused = FOCUS.is_focused(args.widget_id).get();
            if args.focus {
                if !is_focused {
                    FOCUS.focus_widget(args.widget_id, false);
                }
            } else if is_focused {
                FOCUS.focus_exit();
            }
            true
        })
        .perm();

    ACCESS_FOCUS_NAV_ORIGIN_EVENT
        .hook(|args| {
            FOCUS.navigation_origin().set(Some(args.widget_id));
            true
        })
        .perm();

    MOUSE_INPUT_EVENT
        .on_event(
            true,
            hn!(|args| {
                let mut s = FOCUS_SV.write();
                FOCUS.focus(request);
            }),
        )
        .perm();
}
fn event(&mut self, update: &mut EventUpdate) {
    let mut request = None;

    if let Some(args) = TOUCH_INPUT_EVENT.on(update) {
        if args.is_touch_start() {
            // start
            request = Some(FocusRequest::direct_or_exit(args.target.widget_id(), true, false));
        }
    } else if let Some(args) = ACCESS_CLICK_EVENT.on(update) {
        // click
        request = Some(FocusRequest::direct_or_exit(args.widget_id, true, false));
    } else if let Some(args) = WINDOW_FOCUS_CHANGED_EVENT.on(update) {
        // foreground window maybe changed
        let mut focus = FOCUS_SV.write();
        if args.new_focus.is_some()
            && let Some(pending) = focus.pending_window_focus.take()
            && args.is_focus(pending.window)
        {
            request = Some(FocusRequest::direct_or_related(
                pending.target,
                pending.nav_origin.is_some(),
                pending.highlight,
            ));
        }
        if request.is_none()
            && let Some(args) = focus.continue_focus()
        {
            self.notify(&mut focus, Some(args));
        }

        if let Some(window_id) = args.closed() {
            for args in focus.cleanup_returns_win_closed(window_id) {
                RETURN_FOCUS_CHANGED_EVENT.notify(args);
            }
        }
    } else if let Some(args) = RAW_KEY_INPUT_EVENT.on(update) {
        FOCUS_SV.write().last_keyboard_event = args.timestamp;
    }

    if let Some(request) = request {
        let mut focus = FOCUS_SV.write();
        if !matches!(&focus.request, PendingFocusRequest::Update(_)) {
            focus.request = PendingFocusRequest::None;
            focus.pending_highlight = false;
            focus.pending_window_focus = None;
            let args = focus.fulfill_request(request, false);
            self.notify(&mut focus, args);
        }
    }
}

enum PendingFocusRequest {
    None,
    InfoRetry(FocusRequest, DInstant),
    Update(FocusRequest),
    RetryUpdate(FocusRequest),
}
impl PendingFocusRequest {
    fn take_update(&mut self) -> Option<(FocusRequest, bool)> {
        match mem::replace(self, PendingFocusRequest::None) {
            PendingFocusRequest::Update(r) => Some((r, false)),
            PendingFocusRequest::RetryUpdate(r) => Some((r, true)),
            r => {
                *self = r;
                None
            }
        }
    }
    fn take_info(&mut self) -> Option<FocusRequest> {
        match mem::replace(self, PendingFocusRequest::None) {
            PendingFocusRequest::InfoRetry(r, i) => {
                if i.elapsed() < 100.ms() {
                    Some(r)
                } else {
                    None
                }
            }
            r => {
                *self = r;
                None
            }
        }
    }
}

struct PendingWindowFocus {
    window: WindowId,
    target: WidgetId,
    highlight: bool,
    nav_origin: Option<WidgetId>,
}

struct FocusService {
    commands: FocusCommands,
    pending_render: Option<WidgetInfoTree>,

    auto_highlight: Var<Option<Duration>>,
    last_keyboard_event: DInstant,

    focus_disabled_widgets: Var<bool>,
    focus_hidden_widgets: Var<bool>,

    request: PendingFocusRequest,

    focused_var: Var<Option<InteractionPath>>,
    focused: Option<FocusedInfo>,
    navigation_origin_var: Var<Option<WidgetId>>,
    navigation_origin: Option<WidgetId>,

    return_focused_var: IdMap<WidgetId, Var<Option<InteractionPath>>>,
    return_focused: IdMap<WidgetId, InteractionPath>,

    alt_return_var: Var<Option<InteractionPath>>,
    alt_return: Option<(InteractionPath, InteractionPath)>,

    is_highlighting_var: Var<bool>,
    is_highlighting: bool,

    enabled_nav: EnabledNavWithFrame,

    pending_window_focus: Option<PendingWindowFocus>,
    pending_highlight: bool,
}
impl FocusService {
    #[must_use]
    fn new() -> Self {
        Self {
            commands: FocusCommands::new(),
            pending_render: None,

            auto_highlight: var(Some(300.ms())),
            last_keyboard_event: DInstant::EPOCH,

            focus_disabled_widgets: var(true),
            focus_hidden_widgets: var(true),

            request: PendingFocusRequest::None,

            focused_var: var(None),
            focused: None,
            navigation_origin_var: var(None),
            navigation_origin: None,

            return_focused_var: IdMap::default(),
            return_focused: IdMap::default(),

            alt_return_var: var(None),
            alt_return: None,

            is_highlighting_var: var(false),
            is_highlighting: false,

            enabled_nav: EnabledNavWithFrame::invalid(),

            pending_window_focus: None,
            pending_highlight: false,
        }
    }

    fn auto_highlight(&self, timestamp: DInstant) -> bool {
        if let Some(dur) = self.auto_highlight.get()
            && timestamp.duration_since(self.last_keyboard_event) <= dur
        {
            return true;
        }
        false
    }

    fn update_enabled_nav_with_origin(&mut self) {
        let mut origin = self
            .focused
            .as_ref()
            .and_then(|f| WINDOWS.widget_tree(f.path.window_id()).ok()?.get(f.path.widget_id()));
        if let Some(id) = self.navigation_origin
            && let Some(focused) = &origin
            && let Some(o) = focused.tree().get(id)
        {
            origin = Some(o);
        }

        if let Some(o) = origin {
            let o = o.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get());
            self.enabled_nav = o.enabled_nav_with_frame();
        } else {
            self.enabled_nav.nav = FocusNavAction::empty();
        }
    }

    #[must_use]
    fn fulfill_request(&mut self, request: FocusRequest, is_info_retry: bool) -> Option<FocusChangedArgs> {
        match request.target {
            FocusTarget::Direct { target } => self.focus_direct(target, false, request.highlight, false, false, request),
            FocusTarget::DirectOrExit { target, navigation_origin } => {
                self.focus_direct(target, navigation_origin, request.highlight, false, true, request)
            }
            FocusTarget::DirectOrEnter { target, navigation_origin } => {
                self.focus_direct(target, navigation_origin, request.highlight, true, false, request)
            }
            FocusTarget::DirectOrRelated { target, navigation_origin } => {
                self.focus_direct(target, navigation_origin, request.highlight, true, true, request)
            }
            move_ => {
                let origin;
                let origin_tree;
                if let Some(o) = self.navigation_origin_var.get() {
                    origin = Some(o);
                    origin_tree = WINDOWS.focused_info();
                    self.navigation_origin_var.set(None);
                    self.navigation_origin = None;
                } else if let Some(prev) = &self.focused {
                    origin = Some(prev.path.widget_id());
                    origin_tree = WINDOWS.widget_tree(prev.path.window_id()).ok();
                } else {
                    origin = None;
                    origin_tree = None;
                }

                if let Some(info) = origin_tree
                    && let Some(origin) = origin
                {
                    if let Some(w) = info.get(origin) {
                        let w = w.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get());
                        if let Some(new_focus) = match move_ {
                            // tabular
                            FocusTarget::Next => w.next_tab(false),
                            FocusTarget::Prev => w.prev_tab(false),
                            FocusTarget::Enter => w.first_tab_descendant(),
                            FocusTarget::Exit => {
                                if self.alt_return.is_some() && (w.is_alt_scope() || w.ancestors().any(|w| w.is_alt_scope())) {
                                    self.new_focus_for_alt_exit(w, is_info_retry, request.highlight)
                                } else {
                                    w.ancestors().next()
                                }
                            }
                            // directional
                            FocusTarget::Up => w.next_up(),
                            FocusTarget::Right => w.next_right(),
                            FocusTarget::Down => w.next_down(),
                            FocusTarget::Left => w.next_left(),
                            // alt
                            FocusTarget::Alt => {
                                if let Some(alt) = w.alt_scope() {
                                    Some(alt)
                                } else if self.alt_return.is_some() {
                                    // Alt toggles when there is no alt scope.

                                    self.new_focus_for_alt_exit(w, is_info_retry, request.highlight)
                                } else {
                                    None
                                }
                            }
                            // cases covered by parent match
                            FocusTarget::Direct { .. }
                            | FocusTarget::DirectOrExit { .. }
                            | FocusTarget::DirectOrEnter { .. }
                            | FocusTarget::DirectOrRelated { .. } => {
                                unreachable!()
                            }
                        } {
                            // found `new_focus`
                            self.enabled_nav = new_focus.enabled_nav_with_frame();
                            self.move_focus(
                                Some(FocusedInfo::new(new_focus)),
                                None,
                                request.highlight,
                                FocusChangedCause::Request(request),
                            )
                        } else {
                            // no `new_focus`, maybe update highlight and widget path.
                            self.continue_focus_highlight(request.highlight)
                        }
                    } else {
                        // widget not found
                        self.continue_focus_highlight(request.highlight)
                    }
                } else {
                    // window not found
                    self.continue_focus_highlight(request.highlight)
                }
            }
        }
    }

    /// Return focus from the alt scope, handles cases when the return focus is temporarily blocked.
    #[must_use]
    fn new_focus_for_alt_exit(&mut self, prev_w: WidgetFocusInfo, is_info_retry: bool, highlight: bool) -> Option<WidgetFocusInfo> {
        let (_, return_path) = self.alt_return.as_ref().unwrap();

        let return_int = return_path.interactivity();
        let return_id = return_path.widget_id();
        let info = prev_w.focus_tree();

        let r = info.get_or_parent(return_path);
        if let Some(w) = &r
            && w.info().id() != return_id
            && !is_info_retry
            && return_int.is_blocked()
        {
            // blocked return may not have unblocked yet

            if let Some(exists) = info.tree().get(return_id) {
                let exists = exists.into_focus_info(info.focus_disabled_widgets(), info.focus_hidden_widgets());
                if !exists.is_focusable() && exists.info().interactivity().is_blocked() {
                    // Still blocked. A common pattern is to set a `modal` filter on the alt-scope
                    // then remove the modal filter when alt-scope loses focus.
                    //
                    // Here we know that the return focus was blocked after the alt got focus, because
                    // blocked widgets can't before return focus, and we know that we are moving focus
                    // to some `r`. So we setup an info retry, the focus will move to `r` momentarily,
                    // exiting the alt-scope, and if it removes the modal filter the focus will return.
                    self.request =
                        PendingFocusRequest::InfoRetry(FocusRequest::direct_or_related(return_id, false, highlight), INSTANT.now());
                }
            }
        }

        r
    }

    /// Checks if `focused()` is still valid, if not moves focus to nearest valid.
    #[must_use]
    fn continue_focus(&mut self) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused
            && let Ok(true) = WINDOWS.is_focused(focused.path.window_id())
        {
            let info = WINDOWS.widget_tree(focused.path.window_id()).unwrap();
            if let Some(widget) = info
                .get(focused.path.widget_id())
                .map(|w| w.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()))
            {
                if widget.is_focusable() {
                    // :-) probably in the same place, maybe moved inside same window.
                    self.enabled_nav = widget.enabled_nav_with_frame();
                    return self.move_focus(
                        Some(FocusedInfo::new(widget)),
                        self.navigation_origin_var.get(),
                        self.is_highlighting,
                        FocusChangedCause::Recovery,
                    );
                } else {
                    // widget no longer focusable
                    if let Some(parent) = widget.parent() {
                        // move to nearest inside focusable parent, or parent
                        let new_focus = parent.nearest(focused.center, Px::MAX).unwrap_or(parent);
                        self.enabled_nav = new_focus.enabled_nav_with_frame();
                        return self.move_focus(
                            Some(FocusedInfo::new(new_focus)),
                            self.navigation_origin_var.get(),
                            self.is_highlighting,
                            FocusChangedCause::Recovery,
                        );
                    } else {
                        // no focusable parent or root
                        return self.focus_focused_window(self.is_highlighting);
                    }
                }
            } else {
                // widget not found
                for &parent in focused.path.ancestors().iter().rev() {
                    if let Some(parent) = info
                        .get(parent)
                        .and_then(|w| w.into_focusable(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()))
                    {
                        // move to nearest inside focusable parent, or parent
                        let new_focus = parent.nearest(focused.center, Px::MAX).unwrap_or(parent);
                        self.enabled_nav = new_focus.enabled_nav_with_frame();
                        return self.move_focus(
                            Some(FocusedInfo::new(new_focus)),
                            self.navigation_origin_var.get(),
                            self.is_highlighting,
                            FocusChangedCause::Recovery,
                        );
                    }
                }
            }
        } // else window not found or not focused
        // else no current focus
        self.focus_focused_window(false)
    }

    #[must_use]
    fn continue_focus_highlight(&mut self, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(mut args) = self.continue_focus() {
            args.highlight = highlight;
            self.is_highlighting = highlight;
            self.is_highlighting_var.set(highlight);
            Some(args)
        } else if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            self.is_highlighting_var.set(highlight);
            let focused = self.focused.as_ref().map(|p| p.path.clone());
            Some(FocusChangedArgs::now(
                focused.clone(),
                focused,
                highlight,
                FocusChangedCause::Recovery,
                self.enabled_nav.nav,
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_direct(
        &mut self,
        widget_id: WidgetId,
        navigation_origin: bool,
        highlight: bool,
        fallback_to_children: bool,
        fallback_to_parents: bool,
        request: FocusRequest,
    ) -> Option<FocusChangedArgs> {
        let mut next_origin = None;
        let mut target = None;
        if let Some(w) = WINDOWS
            .widget_trees()
            .iter()
            .find_map(|info| info.get(widget_id))
            .map(|w| w.into_focus_info(self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()))
        {
            if w.is_focusable() {
                let enable = w.enabled_nav_with_frame();
                target = Some((FocusedInfo::new(w), enable));
            } else if fallback_to_children {
                let enable = if navigation_origin {
                    next_origin = Some(widget_id);
                    Some(w.enabled_nav_with_frame())
                } else {
                    None
                };
                if let Some(w) = w.descendants().next() {
                    let enable = enable.unwrap_or_else(|| w.enabled_nav_with_frame());
                    target = Some((FocusedInfo::new(w), enable));
                }
            } else if fallback_to_parents {
                let enable = if navigation_origin {
                    next_origin = Some(widget_id);
                    Some(w.enabled_nav_with_frame())
                } else {
                    None
                };
                if let Some(w) = w.parent() {
                    let enable = enable.unwrap_or_else(|| w.enabled_nav_with_frame());
                    target = Some((FocusedInfo::new(w), enable));
                }
            }
        }

        if let Some((target, enabled_nav)) = target {
            if let Ok(false) = WINDOWS.is_focused(target.path.window_id()) {
                if request.force_window_focus || WINDOWS.focused_window_id().is_some() {
                    // if can steal focus from other apps or focus is already in another window of the app.
                    WINDOWS.focus(target.path.window_id()).unwrap();
                } else if request.window_indicator.is_some() {
                    // if app does not have focus, focus stealing is not allowed, but a request indicator can be set.
                    WINDOWS
                        .vars(target.path.window_id())
                        .unwrap()
                        .focus_indicator()
                        .set(request.window_indicator);
                }

                // will focus when the window is focused
                self.pending_window_focus = Some(PendingWindowFocus {
                    window: target.path.window_id(),
                    target: target.path.widget_id(),
                    highlight,
                    nav_origin: next_origin,
                });
                self.navigation_origin = next_origin;
                self.navigation_origin_var.set(next_origin);
                None
            } else {
                self.enabled_nav = enabled_nav;
                self.move_focus(Some(target), next_origin, highlight, FocusChangedCause::Request(request))
            }
        } else {
            self.navigation_origin = next_origin;
            self.navigation_origin_var.set(next_origin);
            self.change_highlight(highlight, request)
        }
    }

    #[must_use]
    fn change_highlight(&mut self, highlight: bool, request: FocusRequest) -> Option<FocusChangedArgs> {
        if self.is_highlighting != highlight {
            self.is_highlighting = highlight;
            self.is_highlighting_var.set(highlight);
            let focused = self.focused.as_ref().map(|p| p.path.clone());
            Some(FocusChangedArgs::now(
                focused.clone(),
                focused,
                highlight,
                FocusChangedCause::Request(request),
                self.enabled_nav.nav,
            ))
        } else {
            None
        }
    }

    #[must_use]
    fn focus_focused_window(&mut self, highlight: bool) -> Option<FocusChangedArgs> {
        if let Some(info) = WINDOWS.focused_info() {
            let info = FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get());
            if let Some(root) = info.focusable_root() {
                // found focused window and it is focusable.
                self.enabled_nav = root.enabled_nav_with_frame();
                self.move_focus(Some(FocusedInfo::new(root)), None, highlight, FocusChangedCause::Recovery)
            } else {
                // has focused window but it is not focusable.
                self.enabled_nav = EnabledNavWithFrame::invalid();
                self.move_focus(None, None, false, FocusChangedCause::Recovery)
            }
        } else {
            // no focused window
            self.enabled_nav = EnabledNavWithFrame::invalid();
            self.move_focus(None, None, false, FocusChangedCause::Recovery)
        }
    }

    #[must_use]
    fn move_focus(
        &mut self,
        new_focus: Option<FocusedInfo>,
        new_origin: Option<WidgetId>,
        highlight: bool,
        cause: FocusChangedCause,
    ) -> Option<FocusChangedArgs> {
        let prev_highlight = std::mem::replace(&mut self.is_highlighting, highlight);
        self.is_highlighting_var.set(highlight);

        self.navigation_origin = new_origin;
        if self.navigation_origin_var.get() != new_origin {
            self.navigation_origin_var.set(new_origin);
        }

        let r = if self.focused.as_ref().map(|p| &p.path) != new_focus.as_ref().map(|p| &p.path) {
            let new_focus = new_focus.as_ref().map(|p| p.path.clone());
            let args = FocusChangedArgs::now(
                self.focused.take().map(|p| p.path),
                new_focus.clone(),
                self.is_highlighting,
                cause,
                self.enabled_nav.nav,
            );
            self.focused_var.set(new_focus);
            Some(args)
        } else if prev_highlight != highlight {
            let new_focus = new_focus.as_ref().map(|p| p.path.clone());
            Some(FocusChangedArgs::now(
                new_focus.clone(),
                new_focus,
                highlight,
                cause,
                self.enabled_nav.nav,
            ))
        } else {
            None
        };

        // can be just a center update.
        self.focused = new_focus;

        r
    }

    #[must_use]
    fn move_after_focus(&mut self, is_tab_cycle_reentry: bool, reverse: bool) -> Option<FocusChangedArgs> {
        if let Some(focused) = &self.focused
            && let Some(info) = WINDOWS.focused_info()
            && let Some(widget) =
                FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()).get(focused.path.widget_id())
        {
            if let Some(nested) = widget.nested_window() {
                tracing::debug!("focus nested window {nested:?}");
                let _ = WINDOWS.focus(nested);
            } else if widget.is_scope() {
                let last_focused = |id| self.return_focused.get(&id).map(|p| p.as_path());
                if let Some(widget) = widget.on_focus_scope_move(last_focused, is_tab_cycle_reentry, reverse) {
                    self.enabled_nav = widget.enabled_nav_with_frame();
                    return self.move_focus(
                        Some(FocusedInfo::new(widget)),
                        self.navigation_origin,
                        self.is_highlighting,
                        FocusChangedCause::ScopeGotFocus(reverse),
                    );
                }
            }
        }
        None
    }

    /// Updates `return_focused` and `alt_return` after `focused` changed.
    #[must_use]
    fn update_returns(&mut self, prev_focus: Option<InteractionPath>) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if let Some((scope, _)) = &mut self.alt_return {
            // if we have an `alt_return` check if is still inside an ALT.

            let mut retain_alt = false;
            if let Some(new_focus) = &self.focused {
                if let Some(s) = new_focus.path.ancestor_path(scope.widget_id()) {
                    retain_alt = true; // just a focus move inside the ALT.
                    *scope = s.into_owned();
                } else if let Ok(info) = WINDOWS.widget_tree(new_focus.path.window_id())
                    && let Some(widget) = FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get())
                        .get(new_focus.path.widget_id())
                {
                    let alt_scope = if widget.is_alt_scope() {
                        Some(widget)
                    } else {
                        widget.scopes().find(|s| s.is_alt_scope())
                    };

                    if let Some(alt_scope) = alt_scope {
                        // entered another ALT
                        retain_alt = true;
                        *scope = alt_scope.info().interaction_path();
                    }
                }
            }

            if !retain_alt {
                let (scope, widget_path) = self.alt_return.take().unwrap();
                self.alt_return_var.set(None);
                r.push(ReturnFocusChangedArgs::now(scope, Some(widget_path), None));
            }
        } else if let Some(new_focus) = &self.focused {
            // if we don't have an `alt_return` but focused something, check if focus
            // moved inside an ALT.

            if let Ok(info) = WINDOWS.widget_tree(new_focus.path.window_id())
                && let Some(widget) = FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get())
                    .get(new_focus.path.widget_id())
            {
                let alt_scope = if widget.is_alt_scope() {
                    Some(widget)
                } else {
                    widget.scopes().find(|s| s.is_alt_scope())
                };
                if let Some(alt_scope) = alt_scope {
                    let scope = alt_scope.info().interaction_path();
                    // entered an alt_scope.

                    if let Some(prev) = &prev_focus {
                        // previous focus is the return.
                        r.push(ReturnFocusChangedArgs::now(scope.clone(), None, Some(prev.clone())));
                        self.alt_return = Some((scope, prev.clone()));
                        self.alt_return_var.set(prev.clone());
                    } else if let Some(parent) = alt_scope.parent() {
                        // no previous focus, ALT parent is the return.
                        let parent_path = parent.info().interaction_path();
                        r.push(ReturnFocusChangedArgs::now(scope.clone(), None, Some(parent_path.clone())));
                        self.alt_return = Some((scope, parent_path.clone()));
                        self.alt_return_var.set(parent_path);
                    }
                }
            }
        }

        /*
         *   Update `return_focused`
         */

        if let Some(new_focus) = &self.focused
            && let Ok(info) = WINDOWS.widget_tree(new_focus.path.window_id())
            && let Some(widget) =
                FocusInfoTree::new(info, self.focus_disabled_widgets.get(), self.focus_hidden_widgets.get()).get(new_focus.path.widget_id())
            && !widget.is_alt_scope()
            && widget.scopes().all(|s| !s.is_alt_scope())
        {
            // if not inside ALT, update return for each LastFocused parent scopes.

            for scope in widget
                .scopes()
                .filter(|s| s.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused)
            {
                let scope = scope.info().interaction_path();
                let path = widget.info().interaction_path();
                if let Some(current) = self.return_focused.get_mut(&scope.widget_id()) {
                    if current != &path {
                        let prev = std::mem::replace(current, path);
                        self.return_focused_var.get(&scope.widget_id()).unwrap().set(current.clone());
                        r.push(ReturnFocusChangedArgs::now(scope, Some(prev), Some(current.clone())));
                    }
                } else {
                    self.return_focused.insert(scope.widget_id(), path.clone());
                    match self.return_focused_var.entry(scope.widget_id()) {
                        IdEntry::Occupied(e) => e.get().set(Some(path.clone())),
                        IdEntry::Vacant(e) => {
                            e.insert(var(Some(path.clone())));
                        }
                    }
                    r.push(ReturnFocusChangedArgs::now(scope, None, Some(path)));
                }
            }
        }

        r
    }

    /// Cleanup `return_focused` and `alt_return` after new widget tree.
    #[must_use]
    fn cleanup_returns(&mut self, info: FocusInfoTree) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if self.return_focused_var.len() > 20 {
            self.return_focused_var
                .retain(|_, var| var.strong_count() > 1 || var.with(Option::is_some))
        }

        self.return_focused.retain(|&scope_id, widget_path| {
            if widget_path.window_id() != info.tree().window_id() {
                return true; // retain, not same window.
            }

            let mut retain = false;

            if let Some(widget) = info.tree().get(widget_path.widget_id()) {
                if let Some(scope) = widget
                    .clone()
                    .into_focus_info(info.focus_disabled_widgets(), info.focus_hidden_widgets())
                    .scopes()
                    .find(|s| s.info().id() == scope_id)
                {
                    if scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused {
                        retain = true; // retain, widget still exists in same scope and scope still is LastFocused.

                        let path = widget.interaction_path();
                        if &path != widget_path {
                            // widget moved inside scope.
                            r.push(ReturnFocusChangedArgs::now(
                                scope.info().interaction_path(),
                                Some(widget_path.clone()),
                                Some(path.clone()),
                            ));
                            *widget_path = path;
                        }
                    }
                } else if let Some(scope) = info.get(scope_id)
                    && scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused
                {
                    // widget not inside scope anymore, but scope still exists and is valid.
                    if let Some(first) = scope.first_tab_descendant() {
                        // LastFocused goes to the first descendant as fallback.
                        retain = true;

                        let path = first.info().interaction_path();
                        r.push(ReturnFocusChangedArgs::now(
                            scope.info().interaction_path(),
                            Some(widget_path.clone()),
                            Some(path.clone()),
                        ));
                        *widget_path = path;
                    }
                }
            } else if let Some(parent) = info.get_or_parent(widget_path) {
                // widget not in window anymore, but a focusable parent is..
                if let Some(scope) = parent.scopes().find(|s| s.info().id() == scope_id)
                    && scope.focus_info().scope_on_focus() == FocusScopeOnFocus::LastFocused
                {
                    // ..and the parent is inside the scope, and the scope is still valid.
                    retain = true;

                    let path = parent.info().interaction_path();
                    r.push(ReturnFocusChangedArgs::now(
                        scope.info().interaction_path(),
                        Some(widget_path.clone()),
                        Some(path.clone()),
                    ));
                    *widget_path = path;
                }
            }

            if !retain {
                let scope_path = info.get(scope_id).map(|i| i.info().interaction_path());

                if scope_path.is_some() {
                    match self.return_focused_var.entry(scope_id) {
                        IdEntry::Occupied(e) => {
                            if e.get().strong_count() == 1 {
                                e.remove();
                            } else {
                                e.get().set(None);
                            }
                        }
                        IdEntry::Vacant(_) => {}
                    }
                } else if let Some(var) = self.return_focused_var.remove(&scope_id)
                    && var.strong_count() > 1
                {
                    var.set(None);
                }

                r.push(ReturnFocusChangedArgs::now(scope_path, Some(widget_path.clone()), None));
            }
            retain
        });

        let mut retain_alt = true;
        if let Some((scope, widget_path)) = &mut self.alt_return
            && widget_path.window_id() == info.tree().window_id()
        {
            // we need to update alt_return

            retain_alt = false; // will retain only if still valid

            if let Some(widget) = info.tree().get(widget_path.widget_id()) {
                if !widget
                    .clone()
                    .into_focus_info(info.focus_disabled_widgets(), info.focus_hidden_widgets())
                    .scopes()
                    .any(|s| s.info().id() == scope.widget_id())
                {
                    retain_alt = true; // retain, widget still exists outside of the ALT scope.

                    let path = widget.interaction_path();
                    if &path != widget_path {
                        // widget moved without entering the ALT scope.
                        r.push(ReturnFocusChangedArgs::now(
                            scope.clone(),
                            Some(widget_path.clone()),
                            Some(path.clone()),
                        ));
                        *widget_path = path;
                    }
                }
            } else if let Some(parent) = info.get_or_parent(widget_path) {
                // widget not in window anymore, but a focusable parent is..
                if !parent.scopes().any(|s| s.info().id() == scope.widget_id()) {
                    // ..and the parent is not inside the ALT scope.
                    retain_alt = true;

                    let path = parent.info().interaction_path();
                    r.push(ReturnFocusChangedArgs::now(
                        scope.clone(),
                        Some(widget_path.clone()),
                        Some(path.clone()),
                    ));
                    *widget_path = path.clone();
                    self.alt_return_var.set(path);
                }
            }
        }
        if !retain_alt {
            let (scope_id, widget_path) = self.alt_return.take().unwrap();
            self.alt_return_var.set(None);
            r.push(ReturnFocusChangedArgs::now(scope_id, Some(widget_path), None));
        }

        r
    }

    /// Cleanup `return_focused` and `alt_return` after a window closed.
    #[must_use]
    fn cleanup_returns_win_closed(&mut self, window_id: WindowId) -> Vec<ReturnFocusChangedArgs> {
        let mut r = vec![];

        if self
            .alt_return
            .as_ref()
            .map(|(_, w)| w.window_id() == window_id)
            .unwrap_or_default()
        {
            let (_, widget_path) = self.alt_return.take().unwrap();
            self.alt_return_var.set(None);
            r.push(ReturnFocusChangedArgs::now(None, Some(widget_path), None));
        }

        self.return_focused.retain(|&scope_id, widget_path| {
            let retain = widget_path.window_id() != window_id;

            if !retain {
                let var = self.return_focused_var.remove(&scope_id).unwrap();
                var.set(None);

                r.push(ReturnFocusChangedArgs::now(None, Some(widget_path.clone()), None));
            }

            retain
        });

        r
    }

    fn update_focused_center(&mut self) {
        if let Some(f) = &mut self.focused {
            let bounds = f.bounds_info.inner_bounds();
            if bounds != PxRect::zero() {
                f.center = bounds.center();
            }
        }
    }
}

impl FocusService {
    fn on_info_tree_update(&mut self, tree: WidgetInfoTree) {
        self.update_focused_center();

        // widget tree rebuilt or visibility may have changed, check if focus is still valid
        let args = self.continue_focus();
        self.notify(args);

        // cleanup return focuses.
        for args in self.cleanup_returns(FocusInfoTree::new(
            tree,
            self.focus_disabled_widgets.get(),
            self.focus_hidden_widgets.get(),
        )) {
            RETURN_FOCUS_CHANGED_EVENT.notify(args);
        }
    }

    fn notify(&mut self, args: Option<FocusChangedArgs>) {
        if let Some(mut args) = args {
            if !args.highlight && args.new_focus.is_some() && self.auto_highlight(args.timestamp) {
                args.highlight = true;
                self.is_highlighting = true;
                self.is_highlighting_var.set(true);
            }

            // reentering single child of parent scope that cycles
            let is_tab_cycle_reentry = matches!(args.cause.request_target(), Some(FocusTarget::Prev | FocusTarget::Next))
                && match (&args.prev_focus, &args.new_focus) {
                    (Some(p), Some(n)) => p.contains(n.widget_id()),
                    _ => false,
                };

            let reverse = matches!(args.cause.request_target(), Some(FocusTarget::Prev));
            let prev_focus = args.prev_focus.clone();
            FOCUS_CHANGED_EVENT.notify(args);

            // may have focused scope.
            while let Some(after_args) = self.move_after_focus(is_tab_cycle_reentry, reverse) {
                FOCUS_CHANGED_EVENT.notify(after_args);
            }

            for return_args in self.update_returns(prev_focus) {
                RETURN_FOCUS_CHANGED_EVENT.notify(return_args);
            }
        }

        let commands = self.commands.as_mut().unwrap();
        commands.update_enabled(self.enabled_nav.nav);
    }
}

#[derive(Debug)]
struct FocusedInfo {
    path: InteractionPath,
    bounds_info: WidgetBoundsInfo,
    center: PxPoint,
}
impl FocusedInfo {
    pub fn new(focusable: WidgetFocusInfo) -> Self {
        FocusedInfo {
            path: focusable.info().interaction_path(),
            bounds_info: focusable.info().bounds_info(),
            center: focusable.info().center(),
        }
    }
}

struct EnabledNavWithFrame {
    nav: FocusNavAction,
    spatial_frame_id: FrameId,
    visibility_id: FrameId,
}
impl EnabledNavWithFrame {
    fn invalid() -> Self {
        Self {
            nav: FocusNavAction::empty(),
            spatial_frame_id: FrameId::INVALID,
            visibility_id: FrameId::INVALID,
        }
    }
    fn needs_refresh(&self, tree: &WidgetInfoTree) -> bool {
        let stats = tree.stats();
        stats.bounds_updated_frame != self.spatial_frame_id || stats.vis_updated_frame != self.visibility_id
    }
}
trait EnabledNavWithFrameExt {
    fn enabled_nav_with_frame(&self) -> EnabledNavWithFrame;
}
impl EnabledNavWithFrameExt for WidgetFocusInfo {
    fn enabled_nav_with_frame(&self) -> EnabledNavWithFrame {
        let stats = self.info().tree().stats();
        EnabledNavWithFrame {
            nav: self.enabled_nav(),
            spatial_frame_id: stats.bounds_updated_frame,
            visibility_id: stats.vis_updated_frame,
        }
    }
}
