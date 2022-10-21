//! Context information for app extensions, windows and widgets.

use crate::{
    app::{AppEventSender, LoopTimer},
    event::{EventHandle, EventHandles, Events},
    service::Services,
    timer::Timers,
    units::*,
    var::{VarHandle, VarHandles, Vars},
    widget_info::{WidgetContextInfo, WidgetInfoTree, WidgetPath},
    widget_instance::WidgetId,
    window::{WindowId, WindowMode},
};
use std::{cell::Cell, fmt, ops::Deref, rc::Rc};

mod contextual;
pub use contextual::*;

mod update;
pub use update::*;

mod state;
pub use state::*;

mod trace;
pub use trace::*;

mod value;
pub use value::*;

/// Owner of [`AppContext`] objects.
///
/// You can only have one instance of this at a time per-thread at a time.
pub(crate) struct OwnedAppContext {
    app_state: OwnedStateMap<state_map::App>,
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
            app_state: OwnedStateMap::new(),
            vars: Vars::instance(app_event_sender.clone()),
            events: Events::instance(app_event_sender),
            services: Services::default(),
            timers: Timers::new(),
            updates,
        }
    }

    /// State that lives for the duration of an application, including a headless application.
    pub fn app_state(&self) -> StateMapRef<state_map::App> {
        self.app_state.borrow()
    }

    /// State that lives for the duration of an application, including a headless application.
    pub fn app_state_mut(&mut self) -> StateMapMut<state_map::App> {
        self.app_state.borrow_mut()
    }

    /// Borrow the app context as an [`AppContext`].
    pub fn borrow(&mut self) -> AppContext {
        AppContext {
            app_state: self.app_state.borrow_mut(),
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

    /// Borrow the [`Services`] only.
    pub fn services(&mut self) -> &mut Services {
        &mut self.services
    }

    /// Applies pending `timers`, `sync`, `vars` and `events` and returns the update
    /// requests and a time for the loop to awake and update.
    #[must_use]
    pub fn apply_updates(&mut self) -> ContextUpdates {
        let events = self.events.apply_updates(&self.vars);
        self.vars.apply_updates(&mut self.updates);

        let (update, update_widgets, layout, render) = self.updates.take_updates();

        ContextUpdates {
            events,
            update,
            update_widgets,
            layout,
            render,
        }
    }

    /// Returns next timer or animation tick time.
    pub fn next_deadline(&mut self, timer: &mut LoopTimer) {
        self.timers.next_deadline(timer);
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
            || self.timers.has_pending_updates()
    }
}

/// Full application context.
pub struct AppContext<'a> {
    /// State that lives for the duration of the application.
    pub app_state: StateMapMut<'a, state_map::App>,

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
        window_state: &mut OwnedStateMap<state_map::Window>,
        f: impl FnOnce(&mut WindowContext) -> R,
    ) -> (R, InfoLayoutRenderUpdates) {
        let _span = UpdatesTrace::window_span(window_id);

        self.updates.enter_window_ctx();

        let mut update_state = OwnedStateMap::new();

        let r = f(&mut WindowContext {
            window_id: &window_id,
            window_mode: &window_mode,
            app_state: self.app_state.reborrow(),
            window_state: window_state.borrow_mut(),
            update_state: update_state.borrow_mut(),
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
    pub app_state: StateMapMut<'a, state_map::App>,

    /// State that lives for the duration of the window.
    pub window_state: StateMapMut<'a, state_map::Window>,

    /// State that lives for the duration of the node tree method call in the window.
    ///
    /// This state lives only for the duration of the function `f` call in [`AppContext::window_context`].
    /// Usually `f` calls one of the [`UiNode`](crate::UiNode) methods and [`WidgetContext`] shares this
    /// state so properties and event handlers can use this state to communicate to further nodes along the
    /// update sequence.
    pub update_state: StateMapMut<'a, state_map::Update>,

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
        root_widget_state: &mut OwnedStateMap<state_map::Widget>,
        var_handles: &mut VarHandles,
        event_handles: &mut EventHandles,
        f: impl FnOnce(&mut WidgetContext) -> R,
    ) -> R {
        let widget_id = info_tree.root().widget_id();

        f(&mut WidgetContext {
            path: &mut WidgetContextPath::new(*self.window_id, widget_id),

            info_tree,
            widget_info,
            app_state: self.app_state.reborrow(),
            window_state: self.window_state.reborrow(),
            widget_state: root_widget_state.borrow_mut(),
            update_state: self.update_state.reborrow(),

            handles: WidgetHandles {
                var_handles,
                event_handles,
            },

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
        root_widget_state: &OwnedStateMap<state_map::Widget>,
        f: impl FnOnce(&mut InfoContext) -> R,
    ) -> R {
        f(&mut InfoContext {
            path: &mut WidgetContextPath::new(*self.window_id, info_tree.root().widget_id()),
            info_tree,
            widget_info,
            app_state: self.app_state.as_ref(),
            window_state: self.window_state.as_ref(),
            widget_state: root_widget_state.borrow(),
            update_state: self.update_state.reborrow(),
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
        info_tree: &WidgetInfoTree,
        widget_info: &WidgetContextInfo,
        root_widget_state: &mut OwnedStateMap<state_map::Widget>,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        let widget_id = info_tree.root().widget_id();
        f(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, font_size).with_screen_ppi(screen_ppi),

            path: &mut WidgetContextPath::new(*self.window_id, widget_id),

            info_tree,
            widget_info,
            app_state: self.app_state.reborrow(),
            window_state: self.window_state.reborrow(),
            widget_state: root_widget_state.borrow_mut(),
            update_state: self.update_state.reborrow(),

            vars: self.vars,

            updates: self.updates,
        })
    }

    /// Runs a function `f` in the render context of a widget.
    pub fn render_context<R>(
        &mut self,
        root_widget_id: WidgetId,
        root_widget_state: &OwnedStateMap<state_map::Widget>,
        info_tree: &WidgetInfoTree,
        widget_info: &WidgetContextInfo,
        f: impl FnOnce(&mut RenderContext) -> R,
    ) -> R {
        f(&mut RenderContext {
            path: &mut WidgetContextPath::new(*self.window_id, root_widget_id),
            info_tree,
            widget_info,
            app_state: self.app_state.as_ref(),
            window_state: self.window_state.as_ref(),
            widget_state: root_widget_state.borrow(),
            update_state: self.update_state.reborrow(),
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
    pub app_state: OwnedStateMap<state_map::App>,
    /// The [`window_state`] value. Empty by default.
    ///
    /// [`window_state`]: WidgetContext::window_state
    pub window_state: OwnedStateMap<state_map::Window>,

    /// The [`widget_state`] value. Empty by default.
    ///
    /// [`widget_state`]: WidgetContext::widget_state
    pub widget_state: OwnedStateMap<state_map::Widget>,

    /// The [`update_state`] value. Empty by default.
    ///
    /// WARNING: In a real context this is reset after each update, in this test context the same map is reused
    /// unless you call [`clear`].
    ///
    /// [`update_state`]: WidgetContext::update_state
    /// [`clear`]: OwnedStateMap::clear
    pub update_state: OwnedStateMap<state_map::Update>,

    /// Var subscriptions storage used in [`WidgetHandles`].
    pub var_handles: VarHandles,
    /// Event subscriptions storage used in [`WidgetHandles`].
    pub event_handles: EventHandles,

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
    pub timers: Timers,

    pub(crate) root_translation_key: crate::render::FrameValueKey<PxTransform>,
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
use crate::{
    widget_info::{WidgetBoundsInfo, WidgetInfoBuilder},
    widget_instance::UiNode,
};
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
            var_handles: Default::default(),
            event_handles: Default::default(),
            services: Services::default(),
            events: Events::instance(sender.clone()),
            vars: Vars::instance(sender.clone()),
            updates: Updates::new(sender),
            timers: Timers::new(),
            root_translation_key: crate::render::FrameValueKey::new_unique(),

            receiver,
            loop_timer: LoopTimer::default(),
        }
    }

    /// Calls `action` in a fake widget context.
    pub fn widget_context<R>(&mut self, action: impl FnOnce(&mut WidgetContext) -> R) -> R {
        action(&mut WidgetContext {
            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            info_tree: &self.info_tree,
            widget_info: &self.widget_info,
            app_state: self.app_state.borrow_mut(),
            window_state: self.window_state.borrow_mut(),
            widget_state: self.widget_state.borrow_mut(),
            update_state: self.update_state.borrow_mut(),
            handles: WidgetHandles {
                var_handles: &mut self.var_handles,
                event_handles: &mut self.event_handles,
            },
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
            app_state: self.app_state.borrow(),
            window_state: self.window_state.borrow(),
            widget_state: self.widget_state.borrow(),
            update_state: self.update_state.borrow_mut(),
        })
    }

    /// Builds a info tree.
    pub fn info_tree<R>(
        &mut self,
        root_bounds_info: WidgetBoundsInfo,
        root_border_info: crate::widget_info::WidgetBorderInfo,
        scale_factor: Factor,
        action: impl FnOnce(&mut InfoContext, &mut WidgetInfoBuilder) -> R,
    ) -> (WidgetInfoTree, R) {
        let mut builder = WidgetInfoBuilder::new(self.window_id, self.root_id, root_bounds_info, root_border_info, scale_factor, None);
        let r = self.info_context(|ctx| action(ctx, &mut builder));
        let (t, _) = builder.finalize();
        (t, r)
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
        action: impl FnOnce(&mut LayoutContext) -> R,
    ) -> R {
        action(&mut LayoutContext {
            metrics: &LayoutMetrics::new(scale_factor, viewport_size, root_font_size)
                .with_font_size(font_size)
                .with_screen_ppi(screen_ppi),

            path: &mut WidgetContextPath::new(self.window_id, self.root_id),
            info_tree: &self.info_tree,
            widget_info: &self.widget_info,
            app_state: self.app_state.borrow_mut(),
            window_state: self.window_state.borrow_mut(),
            widget_state: self.widget_state.borrow_mut(),
            update_state: self.update_state.borrow_mut(),
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
            app_state: self.app_state.borrow(),
            window_state: self.window_state.borrow(),
            widget_state: self.widget_state.borrow(),
            update_state: self.update_state.borrow_mut(),
        })
    }

    /// Applies pending, `sync`, `vars`, `events` and takes all the update requests.
    ///
    /// Returns the [`InfoLayoutRenderUpdates`] and [`ContextUpdates`] a full app and window would
    /// use to update the application.
    pub fn apply_updates(&mut self) -> (InfoLayoutRenderUpdates, ContextUpdates) {
        let win_updt = self.updates.exit_window_ctx();

        for ev in self.receiver.try_iter() {
            match ev {
                crate::app::AppEvent::ViewEvent(_) => unimplemented!(),
                crate::app::AppEvent::Event(ev) => self.events.notify(ev.get()),
                crate::app::AppEvent::Var => self.vars.receive_sended_modify(),
                crate::app::AppEvent::Update(targets) => self.updates.recv_update_internal(targets),
                crate::app::AppEvent::ResumeUnwind(p) => std::panic::resume_unwind(p),
            }
        }
        let events = self.events.apply_updates(&self.vars);
        self.vars.apply_updates(&mut self.updates);
        let (update, update_widgets, layout, render) = self.updates.take_updates();

        (
            win_updt,
            ContextUpdates {
                events,
                update,
                update_widgets,
                layout,
                render,
            },
        )
    }

    /// Update timers and animations, returns next wake time.
    pub fn update_timers(&mut self) -> Option<Deadline> {
        self.loop_timer.awake();

        self.timers.apply_updates(&self.vars, &mut self.loop_timer);
        self.vars.update_animations(&mut self.loop_timer);

        self.loop_timer.poll()
    }

    /// Call [`UiNode::init`].
    pub fn init(&mut self, node: &mut impl UiNode) {
        self.widget_context(|ctx| node.init(ctx));
    }

    /// Call [`UiNode::deinit`].
    pub fn deinit(&mut self, node: &mut impl UiNode) {
        self.widget_context(|ctx| node.deinit(ctx));
    }

    /// Call [`UiNode::event`].
    pub fn event(&mut self, node: &mut impl UiNode, update: &mut crate::event::EventUpdate) {
        self.widget_context(|ctx| {
            if update.delivery_list().has_pending_search() {
                update.fulfill_search(Some(ctx.info_tree).into_iter());
            }

            node.event(ctx, update);
        });
    }

    /// Call [`UiNode::update`], provides [`WidgetUpdates`] if needed.
    pub fn update(&mut self, node: &mut impl UiNode, updates: Option<&mut WidgetUpdates>) {
        if let Some(updates) = updates {
            updates.fulfill_search([&self.info_tree].into_iter());
            self.widget_context(|ctx| node.update(ctx, updates));
        } else {
            let id = node.with_context(|ctx| ctx.id).unwrap_or(self.root_id);
            let mut list = UpdateDeliveryList::new_any();
            list.insert_path(&crate::widget_info::WidgetPath::new(self.window_id, [id]));
            list.enter_window(self.window_id);
            self.widget_context(|ctx| node.update(ctx, &mut WidgetUpdates::new(list)));
        }
    }

    /// Call [`UiNode::info`].
    pub fn info(&mut self, node: &impl UiNode, info: &mut WidgetInfoBuilder) {
        self.info_context(|ctx| node.info(ctx, info))
    }

    /// Call [`UiNode::layout`].
    pub fn layout(&mut self, node: &mut impl UiNode, constrains: Option<PxConstrains2d>) -> PxSize {
        let font_size = Length::pt_to_px(14.0, 1.0.fct());

        let viewport = node
            .with_context(|w| w.widget_info.bounds.outer_size())
            .unwrap_or_else(|| PxSize::new(Px(800), Px(600)));

        self.layout_context(font_size, font_size, viewport, 1.0.fct(), 96.0, |ctx| {
            ctx.with_constrains(
                |c| constrains.unwrap_or(c),
                |ctx| crate::widget_info::WidgetLayout::with_root_widget(ctx, 0, |ctx, wl| node.layout(ctx, wl)),
            )
        })
    }

    /// Call [`UiNode::render`].
    pub fn render(&mut self, node: &impl UiNode, frame: &mut crate::render::FrameBuilder) {
        let key = self.root_translation_key;
        self.render_context(|ctx| {
            frame.push_inner(ctx, key, false, |ctx, frame| node.render(ctx, frame));
        });
    }

    /// Call [`UiNode::render_update`].
    pub fn render_update(&mut self, node: &impl UiNode, update: &mut crate::render::FrameUpdate) {
        let key = self.root_translation_key;
        self.render_context(|ctx| {
            update.update_inner(ctx, key, false, |ctx, update| {
                node.render_update(ctx, update);
            });
        });
    }
}

/// Var and event subscription handles managed by the widget.
///
/// These handles are kept in the widget instance and are dropped on deinit.
///
/// You can access the widget handles for a widget in [`WidgetContext::handles`].
pub struct WidgetHandles<'a> {
    /// Var handlers collection.
    pub var_handles: &'a mut VarHandles,
    /// Event handles collection.
    pub event_handles: &'a mut EventHandles,
}
impl<'a> WidgetHandles<'a> {
    /// Keep var subscription handle.
    pub fn push_var(&mut self, other: VarHandle) {
        self.var_handles.push(other);
    }

    /// Keep var subscription handles.
    pub fn push_vars(&mut self, others: VarHandles) {
        self.var_handles.extend(others);
    }

    /// Keep event subscription handle.
    pub fn push_event(&mut self, other: EventHandle) {
        self.event_handles.push(other);
    }

    /// Keep event subscription handles.
    pub fn push_events(&mut self, others: EventHandles) {
        self.event_handles.extend(others);
    }
}

/// Represents an widget context without parent info.
///
/// Can be accessed using [`UiNode::with_context`].
pub struct WidgetNodeContext<'a> {
    /// The widget ID.
    pub id: WidgetId,

    /// The widget's outer, inner, border and render info.
    pub widget_info: &'a WidgetContextInfo,

    /// State that lives for the duration of the widget.
    pub widget_state: StateMapRef<'a, state_map::Widget>,
}

/// Represents an widget context without parent info.
///
/// Can be accessed using [`UiNode::with_context_mut`].
pub struct WidgetNodeMutContext<'a> {
    /// The widget ID.
    pub id: WidgetId,

    /// The widget's outer, inner, border and render info.
    pub widget_info: &'a WidgetContextInfo,

    /// State that lives for the duration of the widget.
    pub widget_state: StateMapMut<'a, state_map::Widget>,

    /// Var and event subscription handles managed by the widget.
    ///
    /// These handles are kept in the widget instance and are dropped on deinit.
    pub handles: WidgetHandles<'a>,
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
    pub app_state: StateMapMut<'a, state_map::App>,

    /// State that lives for the duration of the window.
    pub window_state: StateMapMut<'a, state_map::Window>,

    /// State that lives for the duration of the widget.
    pub widget_state: StateMapMut<'a, state_map::Widget>,

    /// State that lives for the duration of the node tree method call in the window.
    ///
    /// This state lives only for the current [`UiNode`] method call in all nodes
    /// of the window. You can use this to signal properties and event handlers from nodes that
    /// will be updated further then the current one.
    ///
    /// [`UiNode`]: crate::UiNode
    pub update_state: StateMapMut<'a, state_map::Update>,

    /// Var and event subscription handles managed by the widget.
    ///
    /// These handles are kept in the widget instance and are dropped on deinit.
    pub handles: WidgetHandles<'a>,

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
        widget_state: &mut OwnedStateMap<state_map::Widget>,
        var_handles: &mut VarHandles,
        event_handles: &mut EventHandles,
        f: impl FnOnce(&mut WidgetContext) -> R,
    ) -> (R, InfoLayoutRenderUpdates) {
        self.path.push(widget_id);

        let prev_updates = self.updates.enter_widget_ctx();

        let r = f(&mut WidgetContext {
            path: self.path,

            info_tree: self.info_tree,
            widget_info,
            app_state: self.app_state.reborrow(),
            window_state: self.window_state.reborrow(),
            widget_state: widget_state.borrow_mut(),
            update_state: self.update_state.reborrow(),

            handles: WidgetHandles {
                var_handles,
                event_handles,
            },

            vars: self.vars,
            events: self.events,
            services: self.services,

            timers: self.timers,

            updates: self.updates,
        });

        self.path.pop();

        (r, self.updates.exit_widget_ctx(prev_updates))
    }

    /// Returns an [`InfoContext`] generated from `self`.
    pub fn as_info(&mut self) -> InfoContext {
        InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state.as_ref(),
            window_state: self.window_state.as_ref(),
            widget_state: self.widget_state.as_ref(),
            update_state: self.update_state.reborrow(),
        }
    }

    /// Subscribe the widget to receive `var` updates, register the handle in [`handles`].
    ///
    /// [`handles`]: Self::handles
    pub fn sub_var(&mut self, var: &impl crate::var::AnyVar) -> &mut Self {
        let handle = var.subscribe(self.path.widget_id());
        self.handles.push_var(handle);
        self
    }

    /// Subscribe the widget to receive `event` updates, register the handle in [`handles`].
    ///
    /// [`handles`]: Self::handles
    pub fn sub_event<A: crate::event::EventArgs>(&mut self, event: &crate::event::Event<A>) -> &mut Self {
        let handle = event.subscribe(self.path.widget_id());
        self.handles.push_event(handle);
        self
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

    /// Length of the current path.
    pub fn depth(&self) -> usize {
        self.widget_ids.len()
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

/// A widget measure context.
pub struct MeasureContext<'a> {
    /// Contextual layout metrics.
    pub metrics: &'a LayoutMetrics,

    /// Current widget path.
    pub path: &'a mut WidgetContextPath,

    /// Last build widget info tree of the parent window.
    pub info_tree: &'a WidgetInfoTree,

    /// Current widget's outer, inner, border and render info.
    pub widget_info: &'a WidgetContextInfo,

    /// Read-only access to the state that lives for the duration of the application.
    pub app_state: StateMapRef<'a, state_map::App>,

    /// Read-only access to the state that lives for the duration of the window.
    pub window_state: StateMapRef<'a, state_map::Window>,

    /// Read-only access to the state that lives for the duration of the widget.
    pub widget_state: StateMapRef<'a, state_map::Widget>,

    /// State that lives for the duration of the node tree measure in the window.
    ///
    /// This state lives only for the call to [`UiNode::measure`](crate::UiNode::measure) in all nodes of the window.
    /// You can use this to signal nodes that have not measured yet.
    pub update_state: StateMapMut<'a, state_map::Update>,
}
impl<'a> Deref for MeasureContext<'a> {
    type Target = LayoutMetrics;

    fn deref(&self) -> &Self::Target {
        self.metrics
    }
}
impl<'a> MeasureContext<'a> {
    /// Runs a function `f` in a measure context that has the new or modified constrains.
    ///
    /// The `constrains` closure is called to produce the new constrains, the input is the current constrains.
    pub fn with_constrains<R>(
        &mut self,
        constrains: impl FnOnce(PxConstrains2d) -> PxConstrains2d,
        f: impl FnOnce(&mut MeasureContext) -> R,
    ) -> R {
        f(&mut MeasureContext {
            metrics: &self.metrics.clone().with_constrains(constrains),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state.reborrow(),
        })
    }

    /// Runs a function `f` in a measure context that has its max size subtracted by `removed` and its final size added by `removed`.
    pub fn with_sub_size(&mut self, removed: PxSize, f: impl FnOnce(&mut MeasureContext) -> PxSize) -> PxSize {
        self.with_constrains(|c| c.with_less_size(removed), f) + removed
    }

    /// Runs a function `f` in a layout context that has its max size added by `added` and its final size subtracted by `added`.
    pub fn with_add_size(&mut self, added: PxSize, f: impl FnOnce(&mut MeasureContext) -> PxSize) -> PxSize {
        self.with_constrains(|c| c.with_more_size(added), f) - added
    }

    /// Runs a function `f` in a measure context that has the new computed font size.
    pub fn with_font_size<R>(&mut self, font_size: Px, f: impl FnOnce(&mut MeasureContext) -> R) -> R {
        f(&mut MeasureContext {
            metrics: &self.metrics.clone().with_font_size(font_size),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state.reborrow(),
        })
    }

    /// Runs a function `f` in a measure context that has the new computed viewport.
    pub fn with_viewport<R>(&mut self, viewport: PxSize, f: impl FnOnce(&mut MeasureContext) -> R) -> R {
        f(&mut MeasureContext {
            metrics: &self.metrics.clone().with_viewport(viewport),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state.reborrow(),
        })
    }

    /// Runs a function `f` in the measure context of a widget.
    ///
    /// The `reuse` flag indicates if the cached measure or layout size can be returned instead of calling `f`. It should
    /// only be `false` if the widget has a pending layout request.
    ///
    /// Returns the closure `f` result and the updates requested by it.
    ///
    /// [`render_update`]: Updates::render_update
    pub fn with_widget(
        &mut self,
        widget_id: WidgetId,
        widget_info: &WidgetContextInfo,
        widget_state: &OwnedStateMap<state_map::Widget>,
        reuse: bool,
        f: impl FnOnce(&mut MeasureContext) -> PxSize,
    ) -> PxSize {
        let snap = self.metrics.snapshot();
        if reuse {
            let measure_uses = widget_info.bounds.measure_metrics_used();
            if widget_info
                .bounds
                .measure_metrics()
                .map(|m| m.masked_eq(&snap, measure_uses))
                .unwrap_or(false)
            {
                return widget_info.bounds.measure_outer_size();
            }

            let layout_uses = widget_info.bounds.metrics_used();
            if widget_info
                .bounds
                .metrics()
                .map(|m| m.masked_eq(&snap, layout_uses))
                .unwrap_or(false)
            {
                return widget_info.bounds.outer_size();
            }
        }

        self.path.push(widget_id);

        let parent_uses = self.metrics.enter_widget_ctx();

        let size = f(&mut MeasureContext {
            metrics: self.metrics,

            path: self.path,

            info_tree: self.info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: widget_state.borrow(),
            update_state: self.update_state.reborrow(),
        });

        let measure_uses = self.metrics.exit_widget_ctx(parent_uses);
        widget_info.bounds.set_measure_metrics(Some(snap), measure_uses);
        widget_info.bounds.set_measure_outer_size(size);

        self.path.pop();

        size
    }

    /// Returns an [`InfoContext`] generated from `self`.
    pub fn as_info(&mut self) -> InfoContext {
        InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state.reborrow(),
        }
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
    pub app_state: StateMapMut<'a, state_map::App>,

    /// State that lives for the duration of the window.
    pub window_state: StateMapMut<'a, state_map::Window>,

    /// State that lives for the duration of the widget.
    pub widget_state: StateMapMut<'a, state_map::Widget>,

    /// State that lives for the duration of the node tree layout update call in the window.
    pub update_state: StateMapMut<'a, state_map::Update>,

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
            app_state: self.app_state.reborrow(),
            window_state: self.window_state.reborrow(),
            widget_state: self.widget_state.reborrow(),
            update_state: self.update_state.reborrow(),

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
    pub fn with_font_size<R>(&mut self, font_size: Px, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        f(&mut LayoutContext {
            metrics: &self.metrics.clone().with_font_size(font_size),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state.reborrow(),
            window_state: self.window_state.reborrow(),
            widget_state: self.widget_state.reborrow(),
            update_state: self.update_state.reborrow(),

            vars: self.vars,
            updates: self.updates,
        })
    }

    /// Runs a function `f` in a layout context that has the new computed viewport.
    pub fn with_viewport<R>(&mut self, viewport: PxSize, f: impl FnOnce(&mut LayoutContext) -> R) -> R {
        f(&mut LayoutContext {
            metrics: &self.metrics.clone().with_viewport(viewport),

            path: self.path,

            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state.reborrow(),
            window_state: self.window_state.reborrow(),
            widget_state: self.widget_state.reborrow(),
            update_state: self.update_state.reborrow(),

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
        widget_state: &mut OwnedStateMap<state_map::Widget>,
        f: impl FnOnce(&mut LayoutContext) -> R,
    ) -> (R, InfoLayoutRenderUpdates) {
        self.path.push(widget_id);

        let prev_updates = self.updates.enter_widget_ctx();

        let r = f(&mut LayoutContext {
            metrics: self.metrics,

            path: self.path,

            info_tree: self.info_tree,
            widget_info,
            app_state: self.app_state.reborrow(),
            window_state: self.window_state.reborrow(),
            widget_state: widget_state.borrow_mut(),
            update_state: self.update_state.reborrow(),

            vars: self.vars,
            updates: self.updates,
        });

        self.path.pop();

        (r, self.updates.exit_widget_ctx(prev_updates))
    }

    /// Returns an [`InfoContext`] generated from `self`.
    pub fn as_info(&mut self) -> InfoContext {
        InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state.as_ref(),
            window_state: self.window_state.as_ref(),
            widget_state: self.widget_state.as_ref(),
            update_state: self.update_state.reborrow(),
        }
    }

    /// Returns a [`MeasureContext`] generated from `self`.
    pub fn as_measure(&mut self) -> MeasureContext {
        MeasureContext {
            metrics: self.metrics,
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state.as_ref(),
            window_state: self.window_state.as_ref(),
            widget_state: self.widget_state.as_ref(),
            update_state: self.update_state.reborrow(),
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
    pub app_state: StateMapRef<'a, state_map::App>,

    /// Read-only access to the state that lives for the duration of the window.
    pub window_state: StateMapRef<'a, state_map::Window>,

    /// Read-only access to the state that lives for the duration of the widget.
    pub widget_state: StateMapRef<'a, state_map::Widget>,

    /// State that lives for the duration of the node tree render or render update call in the window.
    ///
    /// This state lives only for the call to [`UiNode::render`](crate::UiNode::render) or
    /// [`UiNode::render_update`](crate::UiNode::render_update) method call in all nodes of the window.
    /// You can use this to signal nodes that have not rendered yet.
    pub update_state: StateMapMut<'a, state_map::Update>,
}
impl<'a> RenderContext<'a> {
    /// Runs a function `f` in the render context of a widget.
    pub fn with_widget<R>(
        &mut self,
        widget_id: WidgetId,
        widget_info: &WidgetContextInfo,
        widget_state: &OwnedStateMap<state_map::Widget>,
        f: impl FnOnce(&mut RenderContext) -> R,
    ) -> R {
        self.path.push(widget_id);
        let r = f(&mut RenderContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: widget_state.borrow(),
            update_state: self.update_state.reborrow(),
        });
        self.path.pop();
        r
    }

    /// Returns an [`InfoContext`] generated from `self`.
    pub fn as_info(&mut self) -> InfoContext {
        InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info: self.widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: self.widget_state,
            update_state: self.update_state.reborrow(),
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
    pub app_state: StateMapRef<'a, state_map::App>,

    /// Read-only access to the state that lives for the duration of the window.
    pub window_state: StateMapRef<'a, state_map::Window>,

    /// Read-only access to the state that lives for the duration of the widget.
    pub widget_state: StateMapRef<'a, state_map::Widget>,

    /// State that lives for the duration of the node tree rebuild or subscriptions aggregation call in the window.
    ///
    /// This state lives only for the call to the [`UiNode::info`](crate::UiNode::info) method in all nodes of the window.
    /// You can use this to signal nodes that have not added info yet.
    pub update_state: StateMapMut<'a, state_map::Update>,
}
impl<'a> InfoContext<'a> {
    /// Runs a function `f` in the info context of a widget.
    pub fn with_widget<R>(
        &mut self,
        widget_id: WidgetId,
        widget_info: &WidgetContextInfo,
        widget_state: &OwnedStateMap<state_map::Widget>,
        f: impl FnOnce(&mut InfoContext) -> R,
    ) -> R {
        self.path.push(widget_id);
        let r = f(&mut InfoContext {
            path: self.path,
            info_tree: self.info_tree,
            widget_info,
            app_state: self.app_state,
            window_state: self.window_state,
            widget_state: widget_state.borrow(),
            update_state: self.update_state.reborrow(),
        });
        self.path.pop();
        r
    }
}

/// Layout metrics snapshot.
///
/// A snapshot can be taken using the [`LayoutMetrics::snapshot`], you can also
/// get the metrics used during the last layout of a widget using the [`WidgetBoundsInfo::metrics`] method.
#[derive(Clone, Copy, Debug)]
pub struct LayoutMetricsSnapshot {
    /// The [`constrains`].
    ///
    /// [`constrains`]: LayoutMetrics::constrains
    pub constrains: PxConstrains2d,
    /// The [`font_size`].
    ///
    /// [`font_size`]: LayoutMetrics::font_size
    pub font_size: Px,
    /// The [`root_font_size`].
    ///
    /// [`root_font_size`]: LayoutMetrics::root_font_size
    pub root_font_size: Px,
    /// The [`scale_factor`].
    ///
    /// [`scale_factor`]: LayoutMetrics::scale_factor
    pub scale_factor: Factor,
    /// The [`viewport`].
    ///
    /// [`viewport`]: LayoutMetrics::viewport
    pub viewport: PxSize,
    /// The [`screen_ppi`].
    ///
    /// [`screen_ppi`]: LayoutMetrics::screen_ppi
    pub screen_ppi: f32,
}
impl LayoutMetricsSnapshot {
    /// Gets if all of the fields in `mask` are equal between `self` and `other`.
    pub fn masked_eq(&self, other: &Self, mask: LayoutMask) -> bool {
        (!mask.contains(LayoutMask::CONSTRAINS) || self.constrains == other.constrains)
            && (!mask.contains(LayoutMask::FONT_SIZE) || self.font_size == other.font_size)
            && (!mask.contains(LayoutMask::ROOT_FONT_SIZE) || self.root_font_size == other.root_font_size)
            && (!mask.contains(LayoutMask::SCALE_FACTOR) || self.scale_factor == other.scale_factor)
            && (!mask.contains(LayoutMask::VIEWPORT) || self.viewport == other.viewport)
            && (!mask.contains(LayoutMask::SCREEN_PPI) || about_eq(self.screen_ppi, other.screen_ppi, 0.0001))
    }
}
impl PartialEq for LayoutMetricsSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.constrains == other.constrains
            && self.font_size == other.font_size
            && self.root_font_size == other.root_font_size
            && self.scale_factor == other.scale_factor
            && self.viewport == other.viewport
            && about_eq(self.screen_ppi, other.screen_ppi, 0.0001)
    }
}
impl std::hash::Hash for LayoutMetricsSnapshot {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.constrains.hash(state);
        self.font_size.hash(state);
        self.root_font_size.hash(state);
        self.scale_factor.hash(state);
        self.viewport.hash(state);
        about_eq_hash(self.screen_ppi, 0.0001, state);
    }
}

/// Layout metrics in a [`LayoutContext`].
///
/// The [`LayoutContext`] type dereferences to this one.
#[derive(Debug, Clone)]
pub struct LayoutMetrics {
    use_mask: Rc<Cell<LayoutMask>>,

    s: LayoutMetricsSnapshot,
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
            use_mask: Rc::new(Cell::new(LayoutMask::NONE)),
            s: LayoutMetricsSnapshot {
                constrains: PxConstrains2d::new_fill_size(viewport),
                font_size,
                root_font_size: font_size,
                scale_factor,
                viewport,
                screen_ppi: 96.0,
            },
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

    /// What metrics where requested so far in the widget or descendants.
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
        self.s.constrains
    }

    /// Current computed font size.
    pub fn font_size(&self) -> Px {
        self.register_use(LayoutMask::FONT_SIZE);
        self.s.font_size
    }

    /// Computed font size at the root widget.
    pub fn root_font_size(&self) -> Px {
        self.register_use(LayoutMask::ROOT_FONT_SIZE);
        self.s.root_font_size
    }

    /// Pixel scale factor.
    pub fn scale_factor(&self) -> Factor {
        self.register_use(LayoutMask::SCALE_FACTOR);
        self.s.scale_factor
    }

    /// Computed size of the nearest viewport ancestor.
    ///
    /// This is usually the window content area size, but can be the scroll viewport size or any other
    /// value depending on the implementation of the context widgets.
    pub fn viewport(&self) -> PxSize {
        self.register_use(LayoutMask::VIEWPORT);
        self.s.viewport
    }

    /// Smallest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_min(&self) -> Px {
        self.s.viewport.width.min(self.s.viewport.height)
    }

    /// Largest dimension of the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn viewport_max(&self) -> Px {
        self.s.viewport.width.max(self.s.viewport.height)
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
        self.s.screen_ppi
    }

    /// Sets the [`constrains`] to the value returned by `constrains`. The closure input is the current constrains.
    ///
    /// [`constrains`]: Self::constrains
    pub fn with_constrains(mut self, constrains: impl FnOnce(PxConstrains2d) -> PxConstrains2d) -> Self {
        self.s.constrains = constrains(self.s.constrains);
        self
    }

    /// Sets the [`font_size`].
    ///
    /// [`font_size`]: Self::font_size
    pub fn with_font_size(mut self, font_size: Px) -> Self {
        self.s.font_size = font_size;
        self
    }

    /// Sets the [`viewport`].
    ///
    /// [`viewport`]: Self::viewport
    pub fn with_viewport(mut self, viewport: PxSize) -> Self {
        self.s.viewport = viewport;
        self
    }

    /// Sets the [`scale_factor`].
    ///
    /// [`scale_factor`]: Self::scale_factor
    pub fn with_scale_factor(mut self, scale_factor: Factor) -> Self {
        self.s.scale_factor = scale_factor;
        self
    }

    /// Sets the [`screen_ppi`].
    ///
    /// [`screen_ppi`]: Self::screen_ppi
    pub fn with_screen_ppi(mut self, screen_ppi: f32) -> Self {
        self.s.screen_ppi = screen_ppi;
        self
    }

    /// Clones all current metrics into a [snapshot].
    ///
    /// [snapshot]: LayoutMetricsSnapshot
    pub fn snapshot(&self) -> LayoutMetricsSnapshot {
        self.s
    }

    pub(crate) fn enter_widget_ctx(&self) -> LayoutMask {
        self.use_mask.replace(LayoutMask::NONE)
    }

    pub(crate) fn exit_widget_ctx(&self, parent_use: LayoutMask) -> LayoutMask {
        let wgt_use = self.use_mask.get();
        self.use_mask.set(parent_use | wgt_use);
        wgt_use
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
        self.metrics.register_use(LayoutMask::CONSTRAINS);
        if self.is_width {
            self.metrics.s.constrains.x
        } else {
            self.metrics.s.constrains.y
        }
    }

    /// Viewport length in the selected dimension.
    pub fn viewport_length(&self) -> Px {
        self.metrics.register_use(LayoutMask::VIEWPORT);
        if self.is_width {
            self.metrics.s.viewport.width
        } else {
            self.metrics.s.viewport.height
        }
    }
}
impl<'m> Deref for Layout1dMetrics<'m> {
    type Target = LayoutMetrics;

    fn deref(&self) -> &Self::Target {
        self.metrics
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
