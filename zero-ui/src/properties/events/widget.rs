//! Widget events, [`on_init`](fn@on_init), [`on_update`](fn@on_update), and more.
//!
//! These events map very close to the [`UiNode`] methods. The event handler have non-standard signatures
//! and the event does not consider the widget's [`interactivity`](crate::core::widget_info::WidgetInfo::interactivity).

use std::mem;

use crate::core::{context::*, handler::*, units::*, widget_instance::*, *};

/// Arguments for the [`on_init`](fn@on_init) event.
#[derive(Clone, Debug, Copy)]
pub struct OnInitArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`init`](UiNode::init) event.
///
/// This property calls `handler` when the widget and its content initializes. Note that widgets
/// can be [deinitialized](fn@on_deinit) and reinitialized, so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnInitArgs::count).
/// to determinate if you are in the first init.
///
/// Note that the widget is not in the [`WidgetInfoTree`] when this event happens, you can use [`on_info_init`] for initialization
/// that depends on the widget info.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`on_info_init`]: fn@on_info_init
/// [`WidgetInfoTree`]: crate::core::widget_info::WidgetInfoTree
#[property(EVENT)]
pub fn on_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    #[ui_node(struct OnInitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnInitArgs>,
        count: usize,
    })]
    impl UiNode for OnInitNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.handler.update(ctx);
        }
    }
    OnInitNode { child, handler, count: 0 }
}

/// Preview [`on_init`] event.
///
/// This property calls `handler` when the widget initializes, before the widget content initializes. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`on_init`]: fn@on_init
#[property(EVENT)]
pub fn on_pre_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    #[ui_node(struct OnPreviewInitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnInitArgs>,
        count: usize,
    })]
    impl UiNode for OnPreviewInitNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnInitArgs { count: self.count });

            self.child.init(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.handler.update(ctx);
            self.child.update(ctx, updates);
        }
    }
    OnPreviewInitNode { child, handler, count: 0 }
}

/// Widget inited and info collected event.
///
/// This event fires after the first [`UiNode::info`] construction after [`UiNode::init`]. This event can be used when
/// some widget initialization needs to happen, but the widget must be in the [`WidgetInfoTree`] for it to work.
///
/// # Handlers
///
/// This property accepts any [`WidgetHandler`], including the async handlers. Use one of the handler macros, [`hn!`],
/// [`hn_once!`], [`async_hn!`] or [`async_hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers spawn a task that is associated with the widget, it will only update when the widget updates,
/// so the task *pauses* when the widget is deinited, and is *canceled* when the widget is dropped.
///
/// [`WidgetInfoTree`]: crate::core::widget_info::WidgetInfoTree
#[property(EVENT)]
pub fn on_info_init(child: impl UiNode, handler: impl WidgetHandler<OnInitArgs>) -> impl UiNode {
    #[ui_node(struct OnInfoInitNode {
        child: impl UiNode,
        handler: impl  WidgetHandler<OnInitArgs>,
        count: usize,
        pending: bool,
    })]
    impl UiNode for OnInfoInitNode {
        fn init(&mut self, ctx: &mut WidgetContext) {
            self.child.init(ctx);

            self.pending = true;
            WIDGET.update();
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            if mem::take(&mut self.pending) {
                self.count = self.count.wrapping_add(1);
                self.handler.event(ctx, &OnInitArgs { count: self.count });
            }

            self.handler.update(ctx);
        }
    }
    OnInfoInitNode {
        child,
        handler,
        count: 0,
        pending: true,
    }
}

/// Arguments for the [`on_update`](fn@on_update) event.
#[derive(Clone, Debug, Copy)]
pub struct OnUpdateArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`update`](UiNode::update) event.
///
/// This property calls `handler` every UI update, after the widget content updates. Updates happen in
/// high volume in between idle moments, so the handler code should be considered a *hot-path*.
///
/// # Handlers
///
/// You can use one of the handler macros, [`hn!`] or [`hn_once!`], to declare a handler closure. You must avoid using the async
/// handlers as they cause an update every time the UI task advances from an await point causing another task to spawn.
#[property(EVENT)]
pub fn on_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs>) -> impl UiNode {
    #[ui_node(struct OnUpdateNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnUpdateArgs>,
        count: usize,
    })]
    impl UiNode for OnUpdateNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });
            self.handler.update(ctx);
        }
    }
    OnUpdateNode { child, handler, count: 0 }
}

/// Preview [`on_update`] event.
///
/// This property calls `handler` every time the UI updates, before the widget content updates. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// You can use one of the handler macros, [`hn!`] or [`hn_once!`], to declare a handler closure. You must avoid using the async
/// handlers as they cause an update every time the UI task advances from an await point causing another task to spawn.
///
/// [`on_update`]: fn@on_update
/// [`on_init`]: fn@on_init
#[property(EVENT)]
pub fn on_pre_update(child: impl UiNode, handler: impl WidgetHandler<OnUpdateArgs>) -> impl UiNode {
    #[ui_node(struct OnPreviewUpdateNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnUpdateArgs>,
        count: usize,
    })]
    impl UiNode for OnPreviewUpdateNode {
        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.count = self.count.wrapping_add(1);
            self.handler.update(ctx);
            self.handler.event(ctx, &OnUpdateArgs { count: self.count });

            self.child.update(ctx, updates);
        }
    }
    OnPreviewUpdateNode { child, handler, count: 0 }
}

/// Arguments for the [`on_deinit`](fn@on_deinit) event.
#[derive(Clone, Debug, Copy)]
pub struct OnDeinitArgs {
    /// Number of time the handler was called.
    ///
    /// The number is `1` for the first call.
    pub count: usize,
}

/// Widget [`deinit`](UiNode::deinit) event.
///
/// This property calls `handler` when the widget deinits, after the widget content deinits. Note that
/// widgets can be [reinitialized](fn@on_init) so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnDeinitArgs::count)
/// to determinate if you are in the first deinit.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers do not work here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`WidgetContext::async_task`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
#[property(EVENT)]
pub fn on_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs>) -> impl UiNode {
    #[ui_node(struct OnDeinitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnDeinitArgs>,
        count: usize,
    })]
    impl UiNode for OnDeinitNode {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.child.deinit(ctx);

            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.child.update(ctx, updates);
            self.handler.update(ctx);
        }
    }
    OnDeinitNode { child, handler, count: 0 }
}

/// Preview [`on_update`] event.
///
/// This property calls `handler` every time the UI updates, before the widget content updates. This means
/// that the `handler` is raised before any [`on_init`] handler.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// ## Async
///
/// The async handlers do not work here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but we are deiniting the widget, probably about to drop it. You can start an UI bound
/// async task in the app context using [`WidgetContext::async_task`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
///
/// [`on_update`]: fn@on_update
/// [`on_init`]: fn@on_init
#[property(EVENT)]
pub fn on_pre_deinit(child: impl UiNode, handler: impl WidgetHandler<OnDeinitArgs>) -> impl UiNode {
    #[ui_node(struct OnPreviewDeinitNode {
        child: impl UiNode,
        handler: impl WidgetHandler<OnDeinitArgs>,
        count: usize,
    })]
    impl UiNode for OnPreviewDeinitNode {
        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.count = self.count.wrapping_add(1);
            self.handler.event(ctx, &OnDeinitArgs { count: self.count });

            self.child.deinit(ctx);
        }

        fn update(&mut self, ctx: &mut WidgetContext, updates: &mut WidgetUpdates) {
            self.handler.update(ctx);
            self.child.update(ctx, updates);
        }
    }
    OnPreviewDeinitNode { child, handler, count: 0 }
}

event_property! {
    /// Widget global inner transform changed.
    pub fn transform_changed {
        event: window::TRANSFORM_CHANGED_EVENT,
        args: window::TransformChangedArgs,
    }

    /// Widget global position changed.
    pub fn move {
        event: window::TRANSFORM_CHANGED_EVENT,
        args: window::TransformChangedArgs,
        filter: |_, a| a.offset() != PxVector::zero(),
    }

    /// Widget interactivity changed.
    ///
    /// Note that there are multiple specific events for interactivity changes, [`on_enable`], [`on_disable`], [`on_block`] and [`on_unblock`]
    /// are some of then.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// [`on_enable`]: fn@on_enable
    /// [`on_disable`]: fn@on_disable
    /// [`on_block`]: fn@on_block
    /// [`on_unblock`]: fn@on_unblock
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn interactivity_changed {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
    }

    /// Widget was enabled or disabled.
    ///
    /// Note that this event tracks the *actual* enabled status of the widget, not the *visually enabled* status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn enabled_changed {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.enabled_change(ctx.path.widget_id()).is_some(),
    }

    /// Widget changed to enabled or disabled visuals.
    ///
    /// Note that this event tracks the *visual* enabled status of the widget, not the *actual* status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn vis_enabled_changed {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.vis_enabled_change(ctx.path.widget_id()).is_some(),
    }

    /// Widget interactions where blocked or unblocked.
    ///
    /// Note  that blocked widgets may still be visually enabled, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree, this is because the interactivity *changed*
    /// from `None`, this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_interactivity_changed`] for a more general interactivity event.
    ///
    /// [`on_interactivity_changed`]: fn@on_interactivity_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn blocked_changed {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.blocked_change(ctx.path.widget_id()).is_some(),
    }

    /// Widget normal interactions now enabled.
    ///
    /// Note that this event tracks the *actual* enabled status of the widget, not the *visually enabled* status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts enabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_enabled_changed`] for a more general event.
    ///
    /// [`on_enabled_changed`]: fn@on_enabled_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn enable {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.is_enable(ctx.path.widget_id()),
    }

    /// Widget normal interactions now disabled.
    ///
    /// Note that this event tracks the *actual* enabled status of the widget, not the *visually enabled* status,
    /// see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts disabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_enabled_changed`] for a more general event.
    ///
    /// [`on_enabled_changed`]: fn@on_enabled_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn disable {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.is_disable(ctx.path.widget_id()),
    }

    /// Widget now using the enabled visuals.
    ///
    /// Note that this event tracks the *visual* enabled status of the widget, not the *actual* status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts visually enabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_vis_enabled_changed`] for a more general event.
    ///
    /// [`on_vis_enabled_changed`]: fn@on_vis_enabled_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn vis_enable {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.is_vis_enable(ctx.path.widget_id()),
    }

    /// Widget now using the disabled visuals.
    ///
    /// Note that this event tracks the *visual* enabled status of the widget, not the *actual* status, the widget may
    /// still be blocked, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts visually disabled,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_vis_enabled_changed`] for a more general event.
    ///
    /// [`on_vis_enabled_changed`]: fn@on_vis_enabled_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn vis_disable {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.is_vis_disable(ctx.path.widget_id()),
    }

    /// Widget interactions now blocked.
    ///
    /// Note  that blocked widgets may still be visually enabled, see [`Interactivity`] for more details.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts blocked,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_blocked_changed`] for a more general event.
    ///
    /// [`on_blocked_changed`]: fn@on_blocked_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn block {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.is_block(ctx.path.widget_id()),
    }

    /// Widget interactions now unblocked.
    ///
    /// Note that the widget may still be disabled.
    ///
    /// Note that an event is received when the widget first initializes in the widget info tree if it starts unblocked,
    /// this initial event can be detected using the [`is_new`] method in the args.
    ///
    /// See [`on_blocked_changed`] for a more general event.
    ///
    /// [`on_blocked_changed`]: fn@on_blocked_changed
    /// [`Interactivity`]: crate::core::widget_info::Interactivity
    /// [`is_new`]: window::WidgetInteractivityChangedArgs::is_new
    pub fn unblock {
        event: window::INTERACTIVITY_CHANGED_EVENT,
        args: window::InteractivityChangedArgs,
        filter: |ctx, a| a.is_unblock(ctx.path.widget_id()),
    }
}
