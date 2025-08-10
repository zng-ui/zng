use zng_app::widget::node::UiNodeOpMethod;

use crate::prelude::*;

/// Arguments for the node operation event properties.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct OnNodeOpArgs {
    /// Operation.
    ///
    /// Event args must be static so access to the full [`UiNodeOp`] is not possible, you can quickly
    /// declare a new property with [`property`] and [`match_node`] if you want to affect the widget this way.
    ///
    /// [`UiNodeOp`]: zng_app::widget::node::UiNodeOp
    /// [`match_node`]: zng_app::widget::node::match_node
    pub op: UiNodeOpMethod,
    /// Number of times the handler was called.
    ///
    /// The number is `1` for the first call and is not reset if the widget is re-inited.
    pub count: usize,
    /// Instant the handler was called.
    pub timestamp: DInstant,
}
impl OnNodeOpArgs {
    /// New args.
    pub fn new(op: UiNodeOpMethod, count: usize, timestamp: DInstant) -> Self {
        Self { op, count, timestamp }
    }
    /// New args with timestamp now.
    pub fn now(op: UiNodeOpMethod, count: usize) -> Self {
        Self::new(op, count, INSTANT.now())
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
/// [`NestGroup::EVENT`]: zng_app::widget::builder::NestGroup::EVENT
/// [`hn!`]: zng_app::handler::hn!
/// [`async_hn!`]: zng_app::handler::async_hn!
/// [`hn_once!`]: zng_app::handler::hn_once!
/// [`async_hn_once!`]: zng_app::handler::async_hn_once!
/// [`WidgetHandler`]: zng_app::handler::WidgetHandler
#[property(EVENT)]
pub fn on_node_op(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_node_op_impl(child.into_node(), handler.cfg_boxed(), |_| true)
}
fn on_node_op_impl(
    child: UiNode,
    mut handler: impl WidgetHandler<OnNodeOpArgs>,
    filter: impl Fn(UiNodeOpMethod) -> bool + Send + 'static,
) -> UiNode {
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
/// [`NestGroup::EVENT`]: zng_app::widget::builder::NestGroup::EVENT
/// [`hn!`]: zng_app::handler::hn!
/// [`hn_once!`]: zng_app::handler::hn_once!
/// [`async_hn!`]: zng_app::handler::async_hn!
/// [`async_hn_once!`]: zng_app::handler::async_hn_once!
/// [`WidgetHandler`]: zng_app::handler::WidgetHandler
#[property(EVENT)]
pub fn on_pre_node_op(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_pre_node_op_impl(child.into_node(), handler.cfg_boxed(), |_| true)
}
fn on_pre_node_op_impl(
    child: UiNode,
    mut handler: impl WidgetHandler<OnNodeOpArgs>,
    filter: impl Fn(UiNodeOpMethod) -> bool + Send + 'static,
) -> UiNode {
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

/// Widget initialized.
///
/// This property calls `handler` when the widget and its content initializes. Note that widgets
/// can be reinitialized, so the `handler` can be called more then once,
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
/// [`WidgetInfoTree`]: zng_app::widget::info::WidgetInfoTree
/// [`hn!`]: zng_app::handler::hn!
/// [`hn_once!`]: zng_app::handler::hn_once!
/// [`async_hn!`]: zng_app::handler::async_hn!
/// [`async_hn_once!`]: zng_app::handler::async_hn_once!
/// [`WidgetHandler`]: zng_app::handler::WidgetHandler
#[property(EVENT)]
pub fn on_init(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_node_op_impl(child.into_node(), handler.cfg_boxed(), |op| matches!(op, UiNodeOpMethod::Init))
}

/// Preview [`on_init`] event.
///
/// [`on_init`]: fn@on_init
#[property(EVENT)]
pub fn on_pre_init(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_pre_node_op_impl(child.into_node(), handler.cfg_boxed(), |op| matches!(op, UiNodeOpMethod::Init))
}

/// Widget info is now available.
///
/// This event fires after the first [`UiNode::info`] is built, after [`UiNode::init`]. This event can be used when
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
/// [`WidgetInfoTree`]: zng_app::widget::info::WidgetInfoTree
///
/// [`UiNode::info`]: zng_app::widget::node::UiNode::info
/// [`UiNode::init`]: zng_app::widget::node::UiNode::init
/// [`hn!`]: zng_app::handler::hn!
/// [`hn_once!`]: zng_app::handler::hn_once!
/// [`async_hn!`]: zng_app::handler::async_hn!
/// [`async_hn_once!`]: zng_app::handler::async_hn_once!
/// [`WidgetHandler`]: zng_app::handler::WidgetHandler
#[property(EVENT)]
pub fn on_info_init(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
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
/// high volume in between idle moments, so the handler code should be considered a hot-path.
///
/// # Handlers
///
/// You can use one of the handler macros, [`hn!`] or [`hn_once!`], to declare a handler closure. You must avoid using the async
/// handlers as they cause an update every time the UI task advances from an await point causing another task to spawn.
///
/// [`hn!`]: zng_app::handler::hn!
/// [`hn_once!`]: zng_app::handler::hn_once!
#[property(EVENT)]
pub fn on_update(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_node_op_impl(child.into_node(), handler.cfg_boxed(), |op| matches!(op, UiNodeOpMethod::Update))
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
/// [`hn!`]: zng_app::handler::hn!
/// [`hn_once!`]: zng_app::handler::hn_once!
#[property(EVENT)]
pub fn on_pre_update(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_pre_node_op_impl(child.into_node(), handler.cfg_boxed(), |op| matches!(op, UiNodeOpMethod::Update))
}

/// Widget deinited.
///
/// This property calls `handler` when the widget deinits, after the widget content deinits. Note that
/// widgets can be reinitialized so the `handler` can be called more then once,
/// you can use one of the *once* handlers to only be called once or use the arguments [`count`](OnNodeOpArgs::count)
/// to determinate if you are in the first deinit.
///
/// # Handlers
///
/// This property accepts the [`WidgetHandler`] that are not async. Use one of the handler macros, [`hn!`] or
/// [`hn_once!`], to declare a handler closure.
///
/// Note that async handlers do not work here because widget bound async tasks only advance past the first `.await`
/// during widget updates, but the widget is deinited before that. You can use [`UPDATES.run`] or [`task::spawn`] to start
/// an async task on deinit.
///
/// # Preview
///
/// You can use the [`on_pre_deinit`] event to receive this event before the widget content deinits.
///
/// [`UPDATES.run`]: zng_app::update::UPDATES::run
/// [`on_pre_deinit`]: fn@on_pre_deinit
/// [`WidgetHandler`]: zng_app::handler::WidgetHandler
/// [`hn!`]: zng_app::handler::hn!
/// [`hn_once!`]: zng_app::handler::hn_once!
/// [`task::spawn`]: zng_task::spawn
#[property(EVENT)]
pub fn on_deinit(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_node_op_impl(child.into_node(), handler.cfg_boxed(), |op| matches!(op, UiNodeOpMethod::Deinit))
}

/// Preview [`on_deinit`] event.
///
/// [`on_deinit`]: fn@on_deinit
#[property(EVENT)]
pub fn on_pre_deinit(child: impl IntoUiNode, handler: impl WidgetHandler<OnNodeOpArgs>) -> UiNode {
    on_pre_node_op_impl(child.into_node(), handler.cfg_boxed(), |op| matches!(op, UiNodeOpMethod::Deinit))
}

/// If the widget has been initialized.
///
/// The `state` is set to `true` on init and to `false` on deinit. This property is useful for
/// declaring transition animations that play on init using `when` blocks.
#[property(CONTEXT)]
pub fn is_inited(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    let state = state.into_var();
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            state.set(true);
        }
        UiNodeOp::Deinit => {
            state.set(false);
        }
        _ => {}
    })
}
