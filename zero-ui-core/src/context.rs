//! Context information for app extensions, windows and widgets.

use crate::{WidgetPath, event::Events, service::Services, units::*, var::Vars, window::WindowId, WidgetId};
use std::cell::Cell;
use std::{fmt, ops::Deref};

use crate::app::{AppEventSender, LoopTimer};

use crate::timer::Timers;
use crate::widget_info::{WidgetContextInfo, WidgetInfoTree};
use crate::{var::VarsRead, window::WindowMode};

mod contextual;
pub use contextual::*;

mod update;
pub use update::*;

mod state;
pub use state::*;

mod trace;
pub use trace::*;

/// Owner of [`AppContext`] objects.
///
/// You can only have one instance of this at a time per-thread at a time.
pub(crate) struct OwnedAppContext {
    app_state: StateMap,
    vars: Vars,
    events: Events,
    services: Services,
    timers: Timers,
    updates: Updates,
}
impl OwnedAppContext {
    /// Produces the single instance of `AppContext` for a normal app run.
    pub fn instance(app_event_sender: AppEventSender) -> Self {
        let updates = Updates::new(app_event_sender.clone());
        OwnedAppContext {
            app_state: StateMap::new(),
            vars: Vars::instance(app_event_sender.clone()),
            events: Events::instance(app_event_sender),
            services: Services::default(),
            timers: Timers::new(),
            updates,
        }
    }

    /// State that lives for the duration of an application, including a headless application.
    pub fn app_state(&self) -> &StateMap {
        &self.app_state
    }

    /// State that lives for the duration of an application, including a headless application.
    pub fn app_state_mut(&mut self) -> &mut StateMap {
        &mut self.app_state
    }

    /// Borrow the app context as an [`AppContext`].
    pub fn borrow(&mut self) -> AppContext {
        AppContext {
            app_state: &mut self.app_state,
            vars: &self.vars,
            events: &mut self.events,
            services: &mut self.services,
            timers: &mut self.timers,
            updates: &mut self.updates,
        }
    }

    /// Borrow the [`Vars`] only.
    pub fn vars(&self) -> &Vars {
        &self.vars
    }

    /// Applies pending `timers`, `sync`, `vars` and `events` and returns the update
    /// requests and a time for the loop to awake and update.
    #[must_use]
    pub fn apply_updates(&mut self) -> ContextUpdates {
        let events = self.events.apply_updates(&self.vars);
        self.vars.apply_updates(&mut self.updates);

        let (update, layout, render) = self.updates.take_updates();

        ContextUpdates {
            events,
            update,
            layout,
            render,
        }
    }

    /// Returns next timer or animation tick time.
    pub fn next_deadline(&mut self, timer: &mut LoopTimer) {
        self.timers.next_deadline(&self.vars, timer);
        self.vars.next_deadline(timer);
    }

    /// Update timers and animations, returns next wake time.
    pub fn update_timers(&mut self, timer: &mut LoopTimer) {
        self.timers.apply_updates(&self.vars, timer);
        self.vars.update_animations(timer);
    }

    /// If a call to `apply_updates` will generate updates (ignoring timers).
    #[must_use]
    pub fn has_pending_updates(&mut self) -> bool {
        self.updates.update_requested()
            || self.updates.layout_requested()
            || self.updates.render_requested()
            || self.vars.has_pending_updates()
            || self.events.has_pending_updates()
    }
}

/// Full application context.
pub struct AppContext<'a> {
    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// Access to variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a mut Events,
    /// Access to application services.
    pub services: &'a mut Services,

    /// Event loop based timers.
    pub timers: &'a mut Timers,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}
impl<'a> AppContext<'a> {
    /// Runs a function `f` in the context of a window.
    ///
    /// Returns the function result and
    pub fn window_context<R>(
        &mut self,
        window_id: WindowId,
        window_mode: WindowMode,
        window_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut WindowContext) -> R,
    ) -> (R, WindowUpdates) {
        let _span = UpdatesTrace::window_span(window_id);

        self.updates.enter_window_ctx();

        let mut update_state = StateMap::new();

        let r = f(&mut WindowContext {
            window_id: &window_id,
            window_mode: &window_mode,
            app_state: self.app_state,
            window_state: &mut window_state.0,
            update_state: &mut update_state,
            vars: self.vars,
            events: self.events,
            services: self.services,
            timers: self.timers,
            updates: self.updates,
        });

        (r, self.updates.exit_window_ctx())
    }
}

/// A window context.
pub struct WindowContext<'a> {
    /// Id of the context window.
    pub window_id: &'a WindowId,

    /// Window mode, headed or not, renderer or not.
    pub window_mode: &'a WindowMode,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the node tree method call in the window.
    ///
    /// This state lives only for the duration of the function `f` call in [`AppContext::window_context`].
    /// Usually `f` calls one of the [`UiNode`](crate::UiNode) methods and [`WidgetContext`] shares this
    /// state so properties and event handlers can use this state to communicate to further nodes along the
    /// update sequence.
    pub update_state: &'a mut StateMap,

    /// Access to variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a mut Events,
    /// Access to application services.
    pub services: &'a mut Services,

    /// Event loop based timers.
    pub timers: &'a mut Timers,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}
impl<'a> WindowContext<'a> {
    /// Runs a function `f` in the context of a widget.
    pub fn widget_context<R>(
        &mut self,
        info_tree: &WidgetInfoTree,
        widget_info: &WidgetContextInfo,
        root_widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut WidgetContext) -> R,
    ) -> R {
        let widget_id = info_tree.root().widget_id();

        #[cfg(not(inspector))]
        let _span = UpdatesTrace::widget_span(widget_id, "", "");
        f(&mut WidgetContext {
            path: &mut WidgetContextPath::new(*self.window_id, widget_id),

            info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &mut root_widget_state.0,
            update_state: self.update_state,

            vars: self.vars,
            events: self.events,
            services: self.services,

            timers: self.timers,

            updates: self.updates,
        })
    }

    /// Run a function `f` in the info context of a widget.
    pub fn info_context<R>(
        &mut self,
        info_tree: &WidgetInfoTree,
        widget_info: &WidgetContextInfo,
        root_widget_state: &OwnedStateMap,
        f: impl FnOnce(&mut InfoContext) -> R,
    ) -> R {
        f(&mut InfoContext {
            path: &mut WidgetContextPath::new(*self.window_id, info_tree.root().widget_id()),
            info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &root_widget_state.0,
            update_state: self.update_state,
            vars: self.vars,
        })
    }

    /// Runs a function `f` in the layout context of a widget.
    #[allow(clippy::too_many_arguments)]
    pub fn layout_context<R>(
        &mut self,
        font_size: Px,
        scale_factor: Factor,
        screen_ppi: f32,
        viewport_size: PxSize,
        metrics_diff: LayoutMask,
        info_tree: &WidgetInfoTree,
        widget_info: &WidgetContextInfo,
        root_widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        let widget_id = info_tree.root().widget_id();
        #[cfg(not(inspector))]
        let _span = UpdatesTrace::widget_span(widget_id, "", "");
        f(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, font_size)
                .with_screen_ppi(screen_ppi)
                .with_diff(metrics_diff),

            path: &mut WidgetContextPath::new(*self.window_id, widget_id),

            info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &mut root_widget_state.0,
            update_state: self.update_state,

            vars: self.vars,

            updates: self.updates,
        })
    }

    /// Runs a function `f` in the render context of a widget.
    pub fn render_context<R>(
        &mut self,
        root_widget_id: WidgetId,
        root_widget_state: &OwnedStateMap,
        info_tree: &WidgetInfoTree,
        widget_info: &WidgetContextInfo,
        f: impl FnOnce(&mut RenderContext) -> R,
    ) -> R {
        f(&mut RenderContext {
            path: &mut WidgetContextPath::new(*self.window_id, root_widget_id),
            info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &root_widget_state.0,
            update_state: self.update_state,
            vars: self.vars,
        })
    }
}

/// A mock [`WidgetContext`] for testing widgets.
///
/// Only a single instance of this type can exist per-thread at a time, see [`new`] for details.
///
/// This is less cumbersome to use then a full headless app, but also more limited. Use a [`HeadlessApp`]
/// for more complex integration tests.
///
/// [`new`]: TestWidgetContext::new
/// [`HeadlessApp`]: crate::app::HeadlessApp
#[cfg(any(test, doc, feature = "test_util"))]
#[cfg_attr(doc_nightly, doc(cfg(feature = "test_util")))]
pub struct TestWidgetContext {
    /// Id of the pretend window that owns the pretend root widget.
    ///
    /// This is a new unique id.
    pub window_id: WindowId,
    /// Id of the pretend root widget that is the context widget.
    pub root_id: WidgetId,

    /// The [`info_tree`] value. Blank by default.
    ///
    /// [`info_tree`]: WidgetContext::info_tree
    pub info_tree: WidgetInfoTree,

    ///The [`widget_info`] value.
    ///
    /// [`widget_info`]: WidgetContext::widget_info
    pub widget_info: WidgetContextInfo,

    /// The [`app_state`] value. Empty by default.
    ///
    /// [`app_state`]: WidgetContext::app_state
    pub app_state: OwnedStateMap,
    /// The [`window_state`] value. Empty by default.
    ///
    /// [`window_state`]: WidgetContext::window_state
    pub window_state: OwnedStateMap,

    /// The [`widget_state`] value. Empty by default.
    ///
    /// [`widget_state`]: WidgetContext::widget_state
    pub widget_state: OwnedStateMap,

    /// The [`update_state`] value. Empty by default.
    ///
    /// WARNING: In a real context this is reset after each update, in this test context the same map is reused
    /// unless you call [`clear`].
    ///
    /// [`update_state`]: WidgetContext::update_state
    /// [`clear`]: OwnedStateMap::clear
    pub update_state: OwnedStateMap,

    /// The [`services`] repository. Empty by default.
    ///
    /// [`services`]: WidgetContext::services
    pub services: Services,

    /// The [`updates`] repository. No request by default.
    ///
    /// WARNING: This is drained of requests after each update, you can do this manually by calling
    /// [`apply_updates`].
    ///
    /// [`updates`]: WidgetContext::updates
    /// [`apply_updates`]: TestWidgetContext::apply_updates
    pub updates: Updates,

    /// The [`vars`] instance.
    ///
    /// [`vars`]: WidgetContext::vars
    pub vars: Vars,

    /// The [`events`] instance. No events registered by default.
    ///
    /// [`events`]: WidgetContext::events
    pub events: Events,

    /// Event loop bases timers.
    ///
    /// TODO testable timers.
    pub timers: Timers,

    pub(crate) root_translation_key: crate::render::FrameBindingKey<RenderTransform>,
    receiver: flume::Receiver<crate::app::AppEvent>,
    loop_timer: crate::app::LoopTimer,
}
#[cfg(any(test, doc, feature = "test_util"))]
impl Default for TestWidgetContext {
    /// [`TestWidgetContext::new`]
    fn default() -> Self {
        Self::new()
    }
}
#[cfg(any(test, doc, feature = "test_util"))]
use crate::widget_info::{WidgetBoundsInfo, WidgetInfoBuilder, WidgetRenderInfo, WidgetSubscriptions};
#[cfg(any(test, doc, feature = "test_util"))]
impl TestWidgetContext {
    /// Gets a new [`TestWidgetContext`] instance. Panics is another instance is alive in the current thread
    /// or if an app is running in the current thread.
    pub fn new() -> Self {
        if crate::app::App::is_running() {
            panic!("only one `TestWidgetContext` or app is allowed per thread")
        }

        let (sender, receiver) = AppEventSender::new();
        let window_id = WindowId::new_unique();
        let root_id = WidgetId::new_unique();
        Self {
            window_id,
            root_id,
            info_tree: WidgetInfoTree::blank(window_id, root_id),
            widget_info: WidgetContextInfo::default(),
            app_state: OwnedStateMap::new(),
            window_state: OwnedStateMap::new(),
            widget_state: OwnedStateMap::new(),
            update_state: OwnedStateMap::new(),
            services: Services::default(),
            events: Events::instance(sender.clone()),
            vars: Vars::instance(sender.clone()),
            updates: Updates::new(sender),
            timers: Timers::new(),
            root_translation_key: crate::render::FrameBindingKey::new_unique(),

            receiver,
            loop_timer: LoopTimer::default(),
        }
    }

    /// Calls `action` in a fake widget context.
    pub fn widget_context<R>(&mut self, action: impl FnOnce(&mut WidgetContext) -> R) -> R {
        #[cfg(not(inspector))]
        let _span = UpdatesTrace::widget_span(self.root_id, "", "");
        action(&mut WidgetContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            info_tree: &self.info_tree,
            widget_info: &self.widget_info,
            app_state: &mut self.app_state.0,
            window_state: &mut self.window_state.0,
            widget_state: &mut self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
            events: &mut self.events,
            services: &mut self.services,
            timers: &mut self.timers,
            updates: &mut self.updates,
        })
    }

    /// Calls `action` in a fake info context.
    pub fn info_context<R>(&mut self, action: impl FnOnce(&mut InfoContext) -> R) -> R {
        action(&mut InfoContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            info_tree: &self.info_tree,
            widget_info: &self.widget_info,
            app_state: &self.app_state.0,
            window_state: &self.window_state.0,
            widget_state: &self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
        })
    }

    /// Builds a info tree.
    pub fn info_tree<R>(
        &mut self,
        root_bounds_info: WidgetBoundsInfo,
        root_border_info: crate::widget_info::WidgetBorderInfo,
        rendered: WidgetRenderInfo,
        action: impl FnOnce(&mut InfoContext, &mut WidgetInfoBuilder) -> R,
    ) -> (WidgetInfoTree, R) {
        let mut builder = WidgetInfoBuilder::new(self.window_id, self.root_id, root_bounds_info, root_border_info, rendered, None);
        let r = self.info_context(|ctx| action(ctx, &mut builder));
        let (t, _) = builder.finalize();
        (t, r)
    }

    /// Aggregate subscriptions.
    pub fn subscriptions<R>(&mut self, action: impl FnOnce(&mut InfoContext, &mut WidgetSubscriptions) -> R) -> (WidgetSubscriptions, R) {
        let mut subs = WidgetSubscriptions::new();
        let r = self.info_context(|ctx| action(ctx, &mut subs));
        (subs, r)
    }

    /// Calls `action` in a fake layout context.
    #[allow(clippy::too_many_arguments)]
    pub fn layout_context<R>(
        &mut self,
        root_font_size: Px,
        font_size: Px,
        viewport_size: PxSize,
        scale_factor: Factor,
        screen_ppi: f32,
        metrics_diff: LayoutMask,
        action: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        action(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, root_font_size)
                .with_font_size(font_size)
                .with_screen_ppi(screen_ppi)
                .with_diff(metrics_diff),

            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            info_tree: &self.info_tree,
            widget_info: &self.widget_info,
            app_state: &mut self.app_state.0,
            window_state: &mut self.window_state.0,
            widget_state: &mut self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
            updates: &mut self.updates,
        })
    }

    /// Calls `action` in a fake render context.
    pub fn render_context<R>(&mut self, action: impl FnOnce(&mut RenderContext) -> R) -> R {
        action(&mut RenderContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            info_tree: &self.info_tree,
            widget_info: &self.widget_info,
            app_state: &self.app_state.0,
            window_state: &self.window_state.0,
            widget_state: &self.widget_state.0,
            update_state: &mut self.update_state.0,
            vars: &self.vars,
        })
    }

    /// Applies pending, `sync`, `vars`, `events` and takes all the update requests.
    ///
    /// Returns the [`WindowUpdates`] and [`ContextUpdates`] a full app and window would
    /// use to update the application.
    pub fn apply_updates(&mut self) -> (WindowUpdates, ContextUpdates) {
        let win_updt = self.updates.exit_window_ctx();

        for ev in self.receiver.try_iter() {
            match ev {
                crate::app::AppEvent::ViewEvent(_) => unimplemented!(),
                crate::app::AppEvent::Event(ev) => self.events.notify_app_event(ev),
                crate::app::AppEvent::Var => self.vars.receive_sended_modify(),
                crate::app::AppEvent::Update(mask) => self.updates.update_internal(mask),
                crate::app::AppEvent::ResumeUnwind(p) => std::panic::resume_unwind(p),
            }
        }
        let events = self.events.apply_updates(&self.vars);
        self.vars.apply_updates(&mut self.updates);
        let (update, layout, render) = self.updates.take_updates();

        (
            win_updt,
            ContextUpdates {
                events,
                update,
                layout,
                render,
            },
        )
    }

    /// Update timers and animations, returns next wake time.
    pub fn update_timers(&mut self) -> Option<std::time::Instant> {
        self.loop_timer.awake();

        self.timers.apply_updates(&self.vars, &mut self.loop_timer);
        self.vars.update_animations(&mut self.loop_timer);

        self.loop_timer.poll()
    }

    /// Force set the current update mask.
    pub fn set_current_update(&mut self, current: crate::widget_info::UpdateMask) {
        self.updates.current = current;
    }
}

/// A widget context.
pub struct WidgetContext<'a> {
    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// Last build widget info tree of the parent window.
    pub info_tree: &'a WidgetInfoTree,

    /// Current widget's outer, inner, border and render info.
    pub widget_info: &'a WidgetContextInfo,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the widget.
    pub widget_state: &'a mut StateMap,

    /// State that lives for the duration of the node tree method call in the window.
    ///
    /// This state lives only for the current [`UiNode`] method call in all nodes
    /// of the window. You can use this to signal properties and event handlers from nodes that
    /// will be updated further then the current one.
    ///
    /// [`UiNode`]: crate::UiNode
    pub update_state: &'a mut StateMap,

    /// Access to variables.
    pub vars: &'a Vars,
    /// Access to application events.
    pub events: &'a mut Events,
    /// Access to application services.
    pub services: &'a mut Services,

    /// Event loop based timers.
    pub timers: &'a mut Timers,

    /// Schedule of actions to apply after this update.
    pub updates: &'a mut Updates,
}
impl<'a> WidgetContext<'a> {
    /// Runs a function `f` in the context of a widget, returns the function result and
    /// what updates where requested inside it.
    pub fn widget_context<R>(
        &mut self,
        widget_id: WidgetId,
        widget_info: &WidgetContextInfo,
        widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut WidgetContext) -> R,
    ) -> (R, WidgetUpdates) {
        #[cfg(not(inspector))]
        let _span = UpdatesTrace::widget_span(widget_id, "", "");

        self.path.push(widget_id);

        let prev_updates = self.updates.enter_widget_ctx();

        let r = self.vars.with_widget(widget_id, || {
            f(&mut WidgetContext {
                path: self.path,

                info_tree: self.info_tree,
                widget_info,
                app_state: self.app_state,
                window_state: self.window_state,
                widget_state: &mut widget_state.0,
                update_state: self.update_state,

                vars: self.vars,
                events: self.events,
                services: self.services,

                timers: self.timers,

                updates: self.updates,
            })
        });

        self.path.pop();

        (r, self.updates.exit_widget_ctx(prev_updates))
    }

    /// Runs an [`InfoContext`] generated from `self`.
    pub fn as_info(&mut self) -> InfoContext {
        InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,
            vars: self.vars,
        }
    }
}

/// Current widget context path.
pub struct WidgetContextPath {
    window_id: WindowId,
    widget_ids: Vec<WidgetId>,
}
impl WidgetContextPath {
    fn new(window_id: WindowId, root_id: WidgetId) -> Self {
        let mut widget_ids = Vec::with_capacity(50);
        widget_ids.push(root_id);
        WidgetContextPath { window_id, widget_ids }
    }

    fn push(&mut self, widget_id: WidgetId) {
        self.widget_ids.push(widget_id);
    }

    fn pop(&mut self) {
        debug_assert!(self.widget_ids.len() > 1, "cannot pop root");
        self.widget_ids.pop();
    }

    /// Parent window id.
    pub fn window_id(&self) -> WindowId {
        self.window_id
    }

    /// Window root widget id.
    pub fn root_id(&self) -> WidgetId {
        self.widget_ids[0]
    }

    /// Current widget id.
    pub fn widget_id(&self) -> WidgetId {
        self.widget_ids[self.widget_ids.len() - 1]
    }

    /// Ancestor widgets, parent first.
    #[allow(clippy::needless_lifetimes)] // clippy bug
    pub fn ancestors<'s>(&'s self) -> impl Iterator<Item = WidgetId> + 's {
        let max = self.widget_ids.len() - 1;
        self.widget_ids[0..max].iter().copied().rev()
    }

    /// Parent widget id.
    pub fn parent(&self) -> Option<WidgetId> {
        self.ancestors().next()
    }

    /// If the `widget_id` is part of the path.
    pub fn contains(&self, widget_id: WidgetId) -> bool {
        self.widget_ids.iter().any(move |&w| w == widget_id)
    }

    /// Returns `true` if the current widget is the window.
    pub fn is_root(&self) -> bool {
        self.widget_ids.len() == 1
    }

    /// If the `path` starts with the current path.
    pub fn is_start_of(&self, path: &WidgetPath) -> bool {
        let len = self.widget_ids.len();
        if path.widgets_path().len() >= len {
            for (cw, pw) in self.widget_ids.iter().rev().zip(path.widgets_path()[..len].iter().rev()) {
                if cw != pw {
                    return false;
                }
            }
            self.window_id() == path.window_id()
        } else {
            false
        }
    }
}
impl fmt::Debug for WidgetContextPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("WidgetContextPath")
                .field("window_id", &self.window_id)
                .field("widget_ids", &self.widget_ids)
                .finish()
        } else {
            write!(f, "{self}")
        }
    }
}
impl fmt::Display for WidgetContextPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // "WinId(1)//Wgt(1)/Wgt(23)"
        write!(f, "{}/", self.window_id)?;
        for w in &self.widget_ids {
            write!(f, "/{w}")?;
        }
        Ok(())
    }
}

/// A widget layout context.
///
/// This type dereferences to [`LayoutMetrics`].
pub struct LayoutContext<'a> {
    /// Contextual layout metrics.
    pub metrics: &'a LayoutMetrics,

    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// Last build widget info tree of the parent window.
    pub info_tree: &'a WidgetInfoTree,

    /// Current widget's outer, inner, border and render info.
    pub widget_info: &'a WidgetContextInfo,

    /// State that lives for the duration of the application.
    pub app_state: &'a mut StateMap,

    /// State that lives for the duration of the window.
    pub window_state: &'a mut StateMap,

    /// State that lives for the duration of the widget.
    pub widget_state: &'a mut StateMap,

    /// State that lives for the duration of the node tree layout update call in the window.
    pub update_state: &'a mut StateMap,

    /// Access to variables.
    ///
    /// Note that if you assign a variable any frame request is deferred and the app loop goes back
    /// to the [`UiNode::update`] cycle.
    ///
    /// [`UiNode::update`]: crate::UiNode::update
    pub vars: &'a Vars,

    /// Updates that can be requested in layout context.
    pub updates: &'a mut LayoutUpdates,
}
impl<'a> Deref for LayoutContext<'a> {
    type Target = LayoutMetrics;

    fn deref(&self) -> &Self::Target {
        self.metrics
    }
}
impl<'a> LayoutContext<'a> {
    /// Runs a function `f` in a layout context that has the new or modified constrains.
    ///
    /// The `constrains` closure is called to produce the new constrains, the input is the current constrains.
    pub fn with_constrains<R>(
        &mut self,
        constrains: impl FnOnce(PxConstrains2d) -> PxConstrains2d,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        f(&mut LayoutContext {
            metrics: &self.metrics.clone().with_constrains(constrains),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,

            vars: self.vars,
            updates: self.updates,
        })
    }

    /// Runs a function `f` in a layout context that has its max size subtracted by `removed` and its final size added by `removed`.
    ///
    /// The constrains are only [peeked], this method does not register a layout dependency on the constrains.
    ///
    /// [peeked]: LayoutMetrics::peek
    pub fn with_sub_size(&mut self, removed: PxSize, f: impl FnOnce(&mut LayoutContext) -> PxSize) -> PxSize {
        self.with_constrains(|c| c.with_less_size(removed), f) + removed
    }

    /// Runs a function `f` in a layout context that has its max size added by `added` and its final size subtracted by `added`.
    ///
    /// The constrains are only [peeked], this method does not register a layout dependency on the constrains.
    ///
    /// [peeked]: LayoutMetrics::peek
    pub fn with_add_size(&mut self, added: PxSize, f: impl FnOnce(&mut LayoutContext) -> PxSize) -> PxSize {
        self.with_constrains(|c| c.with_more_size(added), f) - added
    }

    /// Runs a function `f` in a layout context that has the new computed font size.
    ///
    /// The `font_size_new` flag indicates if the `font_size` value changed from the previous layout call.
    pub fn with_font_size<R>(&mut self, font_size: Px, font_size_new: bool, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        let mut diff = self.metrics.diff;
        diff.set(LayoutMask::FONT_SIZE, font_size_new);
        f(&mut LayoutContext {
            metrics: &self.metrics.clone().with_font_size(font_size).with_diff(diff),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,

            vars: self.vars,
            updates: self.updates,
        })
    }

    /// Runs a function `f` in a layout context that has the new computed viewport.
    ///
    /// The `viewport_new` flag indicates if the `viewport` value changed from the previous layout call.
    pub fn with_viewport<R>(&mut self, viewport: PxSize, viewport_new: bool, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        let mut diff = self.metrics.diff;
        diff.set(LayoutMask::VIEWPORT_SIZE, viewport_new);
        f(&mut LayoutContext {
            metrics: &self.metrics.clone().with_viewport(viewport).with_diff(diff),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,

            vars: self.vars,
            updates: self.updates,
        })
    }

    /// Runs a function `f` in the layout context of a widget.
    ///
    /// Returns the closure `f` result and the updates requested by it.
    ///
    /// [`render_update`]: Updates::render_update
    pub fn with_widget<R>(
        &mut self,
        widget_id: WidgetId,
        widget_info: &WidgetContextInfo,
        widget_state: &mut OwnedStateMap,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> (R, WidgetUpdates) {
        #[cfg(not(inspector))]
        let _span = UpdatesTrace::widget_span(widget_id, "", "");

        self.path.push(widget_id);

        let prev_updates = self.updates.enter_widget_ctx();

        let r = self.vars.with_widget(widget_id, || {
            f(&mut LayoutContext {
                metrics: self.metrics,

                path: self.path,

                info_tree: self.info_tree,
                widget_info,
                app_state: self.app_state,
                window_state: self.window_state,
                widget_state: &mut widget_state.0,
                update_state: self.update_state,

                vars: self.vars,
                updates: self.updates,
            })
        });

        self.path.pop();

        (r, self.updates.exit_widget_ctx(prev_updates))
    }

    /// Runs an [`InfoContext`] generated from `self`.
    pub fn as_info(&mut self) -> InfoContext {
        InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,
            vars: self.vars,
        }
    }
}

/// A widget render context.
pub struct RenderContext<'a> {
    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// Last build widget info tree of the parent window.
    pub info_tree: &'a WidgetInfoTree,

    /// Current widget's outer, inner, border and render info.
    pub widget_info: &'a WidgetContextInfo,

    /// Read-only access to the state that lives for the duration of the application.
    pub app_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the window.
    pub window_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the widget.
    pub widget_state: &'a StateMap,

    /// State that lives for the duration of the node tree render or render update call in the window.
    ///
    /// This state lives only for the call to [`UiNode::render`](crate::UiNode::render) or
    /// [`UiNode::render_update`](crate::UiNode::render_update) method call in all nodes of the window.
    /// You can use this to signal nodes that have not rendered yet.
    pub update_state: &'a mut StateMap,

    /// Read-only access to variables.
    pub vars: &'a VarsRead,
}
impl<'a> RenderContext<'a> {
    /// Runs a function `f` in the render context of a widget.
    pub fn with_widget<R>(
        &mut self,
        widget_id: WidgetId,
        widget_info: &WidgetContextInfo,
        widget_state: &OwnedStateMap,
        f: impl FnOnce(&mut RenderContext) -> R,
    ) -> R {
        self.path.push(widget_id);
        let r = f(&mut RenderContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &widget_state.0,
            update_state: self.update_state,
            vars: self.vars,
        });
        self.path.pop();
        r
    }

    /// Runs an [`InfoContext`] generated from `self`.
    pub fn as_info(&mut self) -> InfoContext {
        InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state,
            vars: self.vars,
        }
    }
}

/// A widget info context.
pub struct InfoContext<'a> {
    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// Last build widget info tree of the parent window.
    pub info_tree: &'a WidgetInfoTree,

    /// Current widget's outer, inner, border and render info.
    pub widget_info: &'a WidgetContextInfo,

    /// Read-only access to the state that lives for the duration of the application.
    pub app_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the window.
    pub window_state: &'a StateMap,

    /// Read-only access to the state that lives for the duration of the widget.
    pub widget_state: &'a StateMap,

    /// State that lives for the duration of the node tree rebuild or subscriptions aggregation call in the window.
    ///
    /// This state lives only for the call to [`UiNode::info`](crate::UiNode::info) or
    /// [`UiNode::subscriptions`](crate::UiNode::subscriptions) method call in all nodes of the window.
    /// You can use this to signal nodes that have not added info yet.
    pub update_state: &'a mut StateMap,

    /// Read-only access to variables.
    pub vars: &'a VarsRead,
}
impl<'a> InfoContext<'a> {
    /// Runs a function `f` in the info context of a widget.
    pub fn with_widget<R>(
        &mut self,
        widget_id: WidgetId,
        widget_info: &WidgetContextInfo,
        widget_state: &OwnedStateMap,
        f: impl FnOnce(&mut InfoContext) -> R,
    ) -> R {
        self.path.push(widget_id);
        let r = f(&mut InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: &widget_state.0,
            update_state: self.update_state,
            vars: self.vars,
        });
        self.path.pop();
        r
    }
}

/// Layout metrics in a [`LayoutContext`].
///
/// The [`LayoutContext`] type dereferences to this one.
#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    use_mask: Cell<LayoutMask>,

    constrains: PxConstrains2d,
    font_size: Px,
    root_font_size: Px,
    scale_factor: Factor,
    viewport: PxSize,
    screen_ppi: f32,
    diff: LayoutMask,
}
impl LayoutMetrics {
    /// New root [`LayoutMetrics`].
    ///
    /// The `font_size` sets both font sizes, the initial PPI is `96.0`, you can use the builder style method and
    /// [`with_screen_ppi`] to set a different value.
    ///
    /// [`with_screen_ppi`]: LayoutMetrics::with_screen_ppi
    pub fn new(scale_factor: Factor, viewport: PxSize, font_size: Px) -> Self {
        LayoutMetrics {
            use_mask: Cell::new(LayoutMask::NONE),
            constrains: PxConstrains2d::new_fill_size(viewport),
            font_size,
            root_font_size: font_size,
            scale_factor,
            viewport,
            screen_ppi: 96.0,
            diff: LayoutMask::all(),
        }
    }

    /// Selects the *width* dimension for 1D metrics.
    pub fn for_x(&self) -> Layout1dMetrics {
        Layout1dMetrics {
            is_width: true,
            metrics: self,
        }
    }

    /// Selects the *height* dimension for 1D metrics.
    pub fn for_y(&self) -> Layout1dMetrics {
        Layout1dMetrics {
            is_width: false,
            metrics: self,
        }
    }

    /// What metrics changed from the last layout in the same context.
    pub fn diff(&self) -> LayoutMask {
        self.diff
    }

    /// What metrics where requested so far.
    pub fn metrics_used(&self) -> LayoutMask {
        self.use_mask.get()
    }

    /// Register that the node layout depends on these contextual values.
    ///
    /// Note that the value methods already register use when they are used.
    pub fn register_use(&self, mask: LayoutMask) {
        let m = self.use_mask.get();
        self.use_mask.set(m | mask);
    }

    /// Get metrics without registering use.
    ///
    /// The `req` closure is called to get a value, then the [`metrics_used`] is undone to the previous state.
    ///
    /// [`metrics_used`]: Self::metrics_used
    pub fn peek<R>(&self, req: impl FnOnce(&Self) -> R) -> R {
        let m = self.use_mask.get();
        let r = req(self);
        self.use_mask.set(m);
        r
    }

    /// Current size constrains.
    pub fn constrains(&self) -> PxConstrains2d {
        self.register_use(LayoutMask::CONSTRAINS);
        self.constrains
    }

    /// Current computed font size.
    pub fn font_size(&self) -> Px {
        self.register_use(LayoutMask::FONT_SIZE);
        self.font_size
    }

    /// Computed font size at the root widget.
    pub fn root_font_size(&self) -> Px {
        self.register_use(LayoutMask::ROOT_FONT_SIZE);
        self.root_font_size
    }

    /// Pixel scale factor.
    pub fn scale_factor(&self) -> Factor {
        self.register_use(LayoutMask::SCALE_FACTOR);
        self.scale_factor
    }

    /// Computed size of the nearest viewport ancestor.
    ///
    /// This is usually the window content area size, but can be the scrollable viewport size or any other
    /// value depending on the implementation of the context widgets.
    pub fn viewport(&self) -> PxSize {
        self.register_use(LayoutMask::VIEWPORT_SIZE);
        self.viewport
    }

    /// Smallest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_min(&self) -> Px {
        self.viewport().width.min(self.viewport.height)
    }

    /// Largest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_max(&self) -> Px {
        self.viewport().width.max(self.viewport.height)
    }

    /// The current screen "pixels-per-inch" resolution.
    ///
    /// This value is dependent in the actual physical size of the screen that the user must manually measure.
    /// For most of the UI you only need the [`scale_factor`].
    ///
    /// If you are implementing some feature like a "print size preview", you need to use this value, and you
    /// can configure a PPI per screen in the [`Monitors`] service.
    ///
    /// Default is `96.0`.
    ///
    /// [`Monitors`]: crate::window::Monitors
    /// [`scale_factor`]: LayoutMetrics::scale_factor
    pub fn screen_ppi(&self) -> f32 {
        self.screen_ppi
    }

    /// Sets the [`constrains`] to the value returned by `constrains`. The closure input is the current constrains.
    ///
    /// [`constrains`]: Self::constrains
    pub fn with_constrains(mut self, constrains: impl FnOnce(PxConstrains2d) -> PxConstrains2d) -> Self {
        self.constrains = constrains(self.constrains);
        self
    }

    /// Sets the [`font_size`].
    ///
    /// [`font_size`]: Self::font_size
    pub fn with_font_size(mut self, font_size: Px) -> Self {
        self.font_size = font_size;
        self
    }

    /// Sets the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn with_viewport(mut self, viewport: PxSize) -> Self {
        self.viewport = viewport;
        self
    }

    /// Sets the [`scale_factor`].
    ///
    /// [`scale_factor`]: Self::scale_factor
    pub fn with_scale_factor(mut self, scale_factor: Factor) -> Self {
        self.scale_factor = scale_factor;
        self
    }

    /// Sets the [`screen_ppi`].
    ///
    /// [`screen_ppi`]: Self::screen_ppi
    pub fn with_screen_ppi(mut self, screen_ppi: f32) -> Self {
        self.screen_ppi = screen_ppi;
        self
    }

    /// Sets the [`diff`].
    ///
    /// [`diff`]: Self::diff
    pub fn with_diff(mut self, diff: LayoutMask) -> Self {
        self.diff = diff;
        self
    }

    /// Sets the [`metrics_used`].
    ///
    /// [`metrics_used`]: Self::metrics_used
    pub fn with_use(mut self, use_mask: LayoutMask) -> Self {
        self.use_mask = Cell::new(use_mask);
        self
    }
}

/// Represents a [`LayoutMetrics`] with a selected dimension.
#[derive(Clone, Copy, Debug)]
pub struct Layout1dMetrics<'m> {
    /// If the selected dimension is *width*, if not it is *height*.
    pub is_width: bool,
    /// The full metrics.
    pub metrics: &'m LayoutMetrics,
}
impl<'m> Layout1dMetrics<'m> {
    /// Length constrains in the selected dimension.
    pub fn constrains(&self) -> PxConstrains {
        if self.is_width {
            self.metrics.constrains.x
        } else {
            self.metrics.constrains.y
        }
    }

    /// Viewport length in the selected dimension.
    pub fn viewport_length(&self) -> Px {
        if self.is_width {
            self.metrics.viewport().width
        } else {
            self.metrics.viewport().height
        }
    }
}
impl<'m> Deref for Layout1dMetrics<'m> {
    type Target = LayoutMetrics;

    fn deref(&self) -> &Self::Target {
        &*self.metrics
    }
}

#[cfg(test)]
pub mod tests {
    use std::rc::Rc;

    use crate::app::App;

    use super::*;

    #[test]
    #[should_panic(expected = "already in `AppContextMut::with`, cannot borrow `&mut AppContext` twice")]
    fn context_reentry() {
        let mut app = App::default().run_headless(false);

        let (scope, ctx) = AppContextScope::new();
        let ctx_a = Rc::new(ctx);
        let ctx_b = Rc::clone(&ctx_a);

        scope.with(&mut app.ctx(), move || {
            ctx_a.with(move |a| {
                ctx_b.with(move |b| {
                    let _invalid: (&mut AppContext, &mut AppContext) = (a, b);
                })
            })
        });
    }
}
