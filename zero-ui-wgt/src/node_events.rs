use std::time::Instant;

use zero_ui_app::widget::instance::UiNodeOpMethod;

use crate::prelude::*;

/// Widget UI node events.
#[widget_mixin]
pub struct WidgetEventMix<P>(P);

/// Represents a node operation.
#[derive(Clone, Debug)]
pub struct OnNodeOpArgs {
    /// Operation.
    ///
    /// Event args must be static so access to the full [`UiNodeOp`] is not possible, you can quickly
    /// declare a new property with [`property`] and [`match_node`] if you want to affect the widget this way.
    pub op: UiNodeOpMethod,
    /// Number of times the handler was called.
    ///
    /// The number is `1` for the first call and is not reset if the widget is re-inited.
    pub count: usize,
    /// Instant the handler was called.
    pub timestamp: Instant,
}
impl OnNodeOpArgs {
    /// New args.
    pub fn new(op: UiNodeOpMethod, count: usize, timestamp: Instant) -> Self {
        Self { op, count, timestamp }
    }
    /// New args with timestamp now.
    pub fn now(op: UiNodeOpMethod, count: usize) -> Self {
        Self::new(op, count, Instant::now())
    }
}

/// On any node operation.
///
/// This property calls `handler` for any widget node operation, after the widget content has processed the operation. This means
/// that the `handler` is raised after any [`on_pre_node_op`] handler. Note that properties of [`NestGroup::EVENT`] or lower
/// can still process the operation before this event.
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
/// [`on_pre_node_op`]: fn@on_pre_node_op
/// [`NestGroup::EVENT`]: zero_ui_app::widget::builder::NestGroup::EVENT
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_node_op(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_node_op_impl(child, handler, |_| true)
}
fn on_node_op_impl(
    child: impl UiNode,
    handler: impl WidgetHandler<OnNodeOpArgs>,
    filter: impl Fn(UiNodeOpMethod) -> bool + Send + 'static,
) -> impl UiNode {
    let mut handler = handler.cfg_boxed();
    let mut count = 1;
    match_node(child, move |child, op| {
        let mtd = op.mtd();
        child.op(op);

        if filter(mtd) {
            handler.event(&OnNodeOpArgs::now(mtd, count));
            count = count.wrapping_add(1);
        }

        if let UiNodeOpMethod::Update = mtd {
            handler.update();
        }
    })
}

/// Preview [`on_node_op`] event.
///
/// This property calls `handler` for any widget node operation, before most of the widget content processes the operation. This means
/// that the `handler` is raised before any [`on_node_op`] handler. Note that properties of [`NestGroup::EVENT`] or lower
/// can still process the operation before this event.
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
/// [`on_node_op`]: fn@on_node_op
/// [`NestGroup::EVENT`]: zero_ui_app::widget::builder::NestGroup::EVENT
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_pre_node_op(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_pre_node_op_impl(child, handler, |_| true)
}
fn on_pre_node_op_impl(
    child: impl UiNode,
    handler: impl WidgetHandler<OnNodeOpArgs>,
    filter: impl Fn(UiNodeOpMethod) -> bool + Send + 'static,
) -> impl UiNode {
    let mut handler = handler.cfg_boxed();
    let mut count = 1;
    match_node(child, move |_, op| {
        if let UiNodeOp::Update { .. } = &op {
            handler.update();
        }

        let mtd = op.mtd();
        if filter(mtd) {
            handler.event(&OnNodeOpArgs::now(mtd, count));
            count = count.wrapping_add(1);
        }
    })
}

/// Widget [`init`](UiNode::init) event.
///
/// This property calls `handler` when the widget and its content initializes. Note that widgets
/// can be [deinitialized](fn@on_deinit) and reinitialized, so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnNodeOpArgs::count).
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
/// [`WidgetInfoTree`]: zero_ui_app::widget::info::WidgetInfoTree
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_init(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_node_op_impl(child, handler, |op| matches!(op, UiNodeOpMethod::Init))
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
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_pre_init(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_pre_node_op_impl(child, handler, |op| matches!(op, UiNodeOpMethod::Init))
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
/// [`WidgetInfoTree`]: zero_ui_app::widget::info::WidgetInfoTree
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_info_init(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    let mut handler = handler.cfg_boxed();
    let mut count = 1;
    enum State {
        WaitInfo,
        InfoInited,
        Done,
    }
    let mut state = State::WaitInfo;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            state = State::WaitInfo;
        }
        UiNodeOp::Info { .. } => {
            if let State::WaitInfo = &state {
                state = State::InfoInited;
                WIDGET.update();
            }
        }
        UiNodeOp::Update { updates } => {
            child.update(updates);

            if let State::InfoInited = &state {
                state = State::Done;
                handler.event(&OnNodeOpArgs::now(UiNodeOpMethod::Update, count));
                count = count.wrapping_add(1);
            }

            handler.update();
        }
        _ => {}
    })
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
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_update(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_node_op_impl(child, handler, |op| matches!(op, UiNodeOpMethod::Update))
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
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_pre_update(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_pre_node_op_impl(child, handler, |op| matches!(op, UiNodeOpMethod::Update))
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
/// async task in the app context using [`UPDATES.run`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_deinit(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_node_op_impl(child, handler, |op| matches!(op, UiNodeOpMethod::Deinit))
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
/// async task in the app context using [`UPDATES.run`] or you can use [`task::spawn`] to start a parallel async task
/// in a worker thread.
///
/// [`on_update`]: fn@on_update
/// [`on_init`]: fn@on_init
#[property(EVENT, widget_impl(WidgetEventMix<P>))]
pub fn on_pre_deinit(child: impl UiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> impl UiNode {
    on_pre_node_op_impl(child, handler, |op| matches!(op, UiNodeOpMethod::Deinit))
}

/// If the widget has inited.
///
/// The `state` is set to `true` on init and to `false` on deinit. This property is useful for
/// declaring transition animations that play on init using `when` blocks.
///
/// # Examples
///
/// Animate a popup when it opens:
///
/// ```
/// use zero_ui::prelude::*;
///
/// # let _ =
/// popup::Popup! {
///     opacity = 0.pct();
///     y = -10;
///     when *#is_inited {
///         #[easing(100.ms())]
///         opacity = 100.pct();
///         #[easing(100.ms())]
///         y = 0;
///     }
///     
///     // ..
/// }
/// # ;
/// ```
#[property(CONTEXT)]
pub fn is_inited(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            let _ = state.set(true);
        }
        UiNodeOp::Deinit => {
            let _ = state.set(false);
        }
        _ => {}
    })
}
