//! Widget, UI node API.

use zero_ui_app_context::context_local;
use zero_ui_state_map::OwnedStateMap;
use std::sync::atomic::Ordering::Relaxed;

use crate::WidgetId;

/// Current context widget.
pub struct WIDGET;
impl WIDGET {
    /// Calls `f` while the widget is set to `ctx`.
    ///
    /// The `ctx` must be `Some(_)`, it will be moved to the [`WIDGET`] storage and back to `ctx` after `f` returns.
    ///
    /// If `update_mode` is [`WidgetUpdateMode::Bubble`] the update flags requested for the `ctx` after `f` will be copied to the
    /// caller widget context, otherwise they are ignored.
    pub fn with_context<R>(&self, ctx: &mut WidgetCtx, update_mode: WidgetUpdateMode, f: impl FnOnce() -> R) -> R {
        let parent_id = WIDGET.try_id();

        if let Some(ctx) = ctx.0.as_mut() {
            ctx.parent_id.store(parent_id, Relaxed);
        } else {
            unreachable!()
        }

        let prev_flags = match update_mode {
            WidgetUpdateMode::Ignore => ctx.0.as_mut().unwrap().flags.load(Relaxed),
            WidgetUpdateMode::Bubble => UpdateFlags::empty(),
        };

        // call `f` in context.
        let r = WIDGET_CTX.with_context(&mut ctx.0, f);

        let ctx = ctx.0.as_mut().unwrap();

        match update_mode {
            WidgetUpdateMode::Ignore => {
                ctx.flags.store(prev_flags, Relaxed);
            }
            WidgetUpdateMode::Bubble => {
                let wgt_flags = ctx.flags.load(Relaxed);

                if let Some(parent) = parent_id.map(|_| WIDGET_CTX.get()) {
                    let propagate = wgt_flags
                        & (UpdateFlags::UPDATE
                            | UpdateFlags::INFO
                            | UpdateFlags::LAYOUT
                            | UpdateFlags::RENDER
                            | UpdateFlags::RENDER_UPDATE);

                    let _ = parent.flags.fetch_update(Relaxed, Relaxed, |mut u| {
                        if !u.contains(propagate) {
                            u.insert(propagate);
                            Some(u)
                        } else {
                            None
                        }
                    });
                    ctx.parent_id.store(None, Relaxed);
                } else if let Some(window_id) = WINDOW.try_id() {
                    // is at root, register `UPDATES`
                    UPDATES.update_flags_root(wgt_flags, window_id, ctx.id);
                    // some builders don't clear the root widget flags like they do for other widgets.
                    ctx.flags.store(UpdateFlags::empty(), Relaxed);
                } else {
                    // used outside window
                    UPDATES.update_flags(wgt_flags, ctx.id);
                    ctx.flags.store(UpdateFlags::empty(), Relaxed);
                }
            }
        }

        r
    }
    #[cfg(any(test, doc, feature = "test_util"))]
    pub(crate) fn test_root_updates(&self) {
        let ctx = WIDGET_CTX.get();
        // is at root, register `UPDATES`
        UPDATES.update_flags_root(ctx.flags.load(Relaxed), WINDOW.id(), ctx.id);
        // some builders don't clear the root widget flags like they do for other widgets.
        ctx.flags.store(UpdateFlags::empty(), Relaxed);
    }

    /// Calls `f` while no widget is available in the context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WIDGET_CTX.with_default(f)
    }

    /// Calls `f` with an override target for var and event subscription handles.
    pub fn with_handles<R>(&self, handles: &mut WidgetHandlesCtx, f: impl FnOnce() -> R) -> R {
        WIDGET_HANDLES_CTX.with_context(&mut handles.0, f)
    }

    /// Returns `true` if called inside a widget.
    pub fn is_in_widget(&self) -> bool {
        !WIDGET_CTX.is_default()
    }

    /// Get the widget ID, if called inside a widget.
    pub fn try_id(&self) -> Option<WidgetId> {
        if self.is_in_widget() {
            Some(WIDGET_CTX.get().id)
        } else {
            None
        }
    }

    /// Gets a text with detailed path to the current widget.
    ///
    /// This can be used to quickly identify the current widget during debug, the path printout will contain
    /// the widget types if the inspector metadata is found for the widget.
    ///
    /// This method does not panic if called outside of an widget.
    pub fn trace_path(&self) -> Txt {
        if let Some(w_id) = WINDOW.try_id() {
            if let Some(id) = self.try_id() {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(id) {
                    wgt.trace_path()
                } else {
                    formatx!("{w_id:?}//<no-info>/{id:?}")
                }
            } else {
                formatx!("{w_id:?}//<no-widget>")
            }
        } else if let Some(id) = self.try_id() {
            formatx!("<no-window>//{id:?}")
        } else {
            Txt::from_str("<no-widget>")
        }
    }

    /// Gets a text with a detailed widget id.
    ///
    /// This can be used to quickly identify the current widget during debug, the printout will contain the widget
    /// type if the inspector metadata is found for the widget.
    ///
    /// This method does not panic if called outside of an widget.
    pub fn trace_id(&self) -> Txt {
        if let Some(id) = self.try_id() {
            if WINDOW.try_id().is_some() {
                let tree = WINDOW.info();
                if let Some(wgt) = tree.get(id) {
                    wgt.trace_id()
                } else {
                    formatx!("{id:?}")
                }
            } else {
                formatx!("{id:?}")
            }
        } else {
            Txt::from("<no-widget>")
        }
    }

    /// Get the widget ID if called inside a widget, or panic.
    pub fn id(&self) -> WidgetId {
        WIDGET_CTX.get().id
    }

    /// Gets the widget info.
    ///
    /// # Panics
    ///
    /// If called before the widget info is inited in the parent window.
    pub fn info(&self) -> WidgetInfo {
        WINDOW.info().get(WIDGET.id()).expect("widget info not init")
    }

    /// Schedule an [`UpdateOp`] for the current widget.
    pub fn update_op(&self, op: UpdateOp) -> &Self {
        match op {
            UpdateOp::Update => self.update(),
            UpdateOp::Info => self.update_info(),
            UpdateOp::Layout => self.layout(),
            UpdateOp::Render => self.render(),
            UpdateOp::RenderUpdate => self.render_update(),
        }
    }

    fn update_impl(&self, flag: UpdateFlags) -> &Self {
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if !f.contains(flag) {
                f.insert(flag);
                Some(f)
            } else {
                None
            }
        });
        self
    }

    /// Schedule an update for the current widget.
    ///
    /// After the current update cycle the app-extensions, parent window and widgets will update again.
    pub fn update(&self) -> &Self {
        self.update_impl(UpdateFlags::UPDATE)
    }

    /// Schedule an info rebuild for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-build the info tree.
    pub fn update_info(&self) -> &Self {
        self.update_impl(UpdateFlags::INFO)
    }

    /// Schedule a re-layout for the current widget.
    ///
    /// After all requested updates apply the parent window and widgets will re-layout.
    pub fn layout(&self) -> &Self {
        self.update_impl(UpdateFlags::LAYOUT)
    }

    /// Schedule a re-render for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will re-render.
    ///
    /// This also overrides any pending [`render_update`] request.
    ///
    /// [`render_update`]: Self::render_update
    pub fn render(&self) -> &Self {
        self.update_impl(UpdateFlags::RENDER)
    }

    /// Schedule a frame update for the current widget.
    ///
    /// After all requested updates and layouts apply the parent window and widgets will update the frame.
    ///
    /// This request is supplanted by any [`render`] request.
    ///
    /// [`render`]: Self::render
    pub fn render_update(&self) -> &Self {
        self.update_impl(UpdateFlags::RENDER_UPDATE)
    }

    /// Flags the widget to re-init after the current update returns.
    ///
    /// The widget responds to this request differently depending on the node method that calls it:
    ///
    /// * [`UiNode::init`] and [`UiNode::deinit`]: Request is ignored, removed.
    /// * [`UiNode::event`]: If the widget is pending a reinit, it is reinited first, then the event is propagated to child nodes.
    ///                      If a reinit is requested during event handling the widget is reinited immediately after the event handler.
    /// * [`UiNode::update`]: If the widget is pending a reinit, it is reinited and the update ignored.
    ///                       If a reinit is requested during update the widget is reinited immediately after the update.
    /// * Other methods: Reinit request is flagged and an [`UiNode::update`] is requested for the widget.
    pub fn reinit(&self) {
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if !f.contains(UpdateFlags::REINIT) {
                f.insert(UpdateFlags::REINIT);
                Some(f)
            } else {
                None
            }
        });
    }

    /// Calls `f` with a read lock on the current widget state map.
    ///
    /// Note that this locks the entire [`WIDGET`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state<R>(&self, f: impl FnOnce(StateMapRef<WIDGET>) -> R) -> R {
        f(WIDGET_CTX.get().state.read().borrow())
    }

    /// Calls `f` with a write lock on the current widget state map.
    ///
    /// Note that this locks the entire [`WIDGET`], this is an entry point for widget extensions and must
    /// return as soon as possible. A common pattern is cloning the stored value.
    pub fn with_state_mut<R>(&self, f: impl FnOnce(StateMapMut<WIDGET>) -> R) -> R {
        f(WIDGET_CTX.get().state.write().borrow_mut())
    }

    /// Get the widget state `id`, if it is set.
    ///
    /// Panics if not called inside a widget.
    pub fn get_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> Option<T> {
        let id = id.into();
        self.with_state(|s| s.get_clone(id))
    }

    /// Require the widget state `id`.
    ///
    /// Panics if the `id` is not set or is not called inside a widget.
    pub fn req_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> T {
        let id = id.into();
        self.with_state(|s| s.req(id).clone())
    }

    /// Set the widget state `id` to `value`.
    ///
    /// Returns the previous set value.
    pub fn set_state<T: StateValue>(&self, id: impl Into<StateId<T>>, value: impl Into<T>) -> Option<T> {
        let id = id.into();
        let value = value.into();
        self.with_state_mut(|mut s| s.set(id, value))
    }

    /// Sets the widget state `id` without value.
    ///
    /// Returns if the state `id` was already flagged.
    pub fn flag_state(&self, id: impl Into<StateId<()>>) -> bool {
        let id = id.into();
        self.with_state_mut(|mut s| s.flag(id))
    }

    /// Calls `init` and sets `id` if the `id` is not already set in the widget.
    pub fn init_state<T: StateValue>(&self, id: impl Into<StateId<T>>, init: impl FnOnce() -> T) {
        let id = id.into();
        self.with_state_mut(|mut s| {
            s.entry(id).or_insert_with(init);
        });
    }

    /// Sets the `id` to the default value if it is not already set.
    pub fn init_state_default<T: StateValue + Default>(&self, id: impl Into<StateId<T>>) {
        self.init_state(id.into(), Default::default)
    }

    /// Returns `true` if the `id` is set or flagged in the widget.
    pub fn contains_state<T: StateValue>(&self, id: impl Into<StateId<T>>) -> bool {
        let id = id.into();
        self.with_state(|s| s.contains(id))
    }

    /// Subscribe to receive [`UpdateOp`] when the `var` changes.
    pub fn sub_var_op(&self, op: UpdateOp, var: &impl AnyVar) -> &Self {
        let w = WIDGET_CTX.get();
        let s = var.subscribe(op, w.id);

        if WIDGET_HANDLES_CTX.is_default() {
            w.handles.var_handles.lock().push(s);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().push(s);
        }

        self
    }

    /// Subscribe to receive [`UpdateOp`] when the `var` changes.
    pub fn sub_var_op_when<T: VarValue>(
        &self,
        op: UpdateOp,
        var: &impl Var<T>,
        predicate: impl Fn(&T) -> bool + Send + Sync + 'static,
    ) -> &Self {
        let w = WIDGET_CTX.get();
        let s = var.subscribe_when(op, w.id, predicate);

        if WIDGET_HANDLES_CTX.is_default() {
            w.handles.var_handles.lock().push(s);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().push(s);
        }

        self
    }

    /// Subscribe to receive updates when the `var` changes.
    pub fn sub_var(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Update, var)
    }
    /// Subscribe to receive updates when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Update, var, predicate)
    }

    /// Subscribe to receive info rebuild requests when the `var` changes.
    pub fn sub_var_info(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Info, var)
    }
    /// Subscribe to receive info rebuild requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_info_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Info, var, predicate)
    }

    /// Subscribe to receive layout requests when the `var` changes.
    pub fn sub_var_layout(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Layout, var)
    }
    /// Subscribe to receive layout requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_layout_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Layout, var, predicate)
    }

    /// Subscribe to receive render requests when the `var` changes.
    pub fn sub_var_render(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::Render, var)
    }
    /// Subscribe to receive render requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_render_when<T: VarValue>(&self, var: &impl Var<T>, predicate: impl Fn(&T) -> bool + Send + Sync + 'static) -> &Self {
        self.sub_var_op_when(UpdateOp::Render, var, predicate)
    }

    /// Subscribe to receive render update requests when the `var` changes.
    pub fn sub_var_render_update(&self, var: &impl AnyVar) -> &Self {
        self.sub_var_op(UpdateOp::RenderUpdate, var)
    }
    /// Subscribe to receive render update requests when the `var` changes and the `predicate` approves the new value.
    ///
    /// Note that the `predicate` does not run in the widget context, it runs on the app context.
    pub fn sub_var_render_update_when<T: VarValue>(
        &self,
        var: &impl Var<T>,
        predicate: impl Fn(&T) -> bool + Send + Sync + 'static,
    ) -> &Self {
        self.sub_var_op_when(UpdateOp::RenderUpdate, var, predicate)
    }

    /// Subscribe to receive events from `event` when the event targets this widget.
    pub fn sub_event<A: EventArgs>(&self, event: &Event<A>) -> &Self {
        let w = WIDGET_CTX.get();
        let s = event.subscribe(w.id);

        if WIDGET_HANDLES_CTX.is_default() {
            w.handles.event_handles.lock().push(s);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().push(s);
        }

        self
    }

    /// Hold the `handle` until the widget is deinited.
    pub fn push_event_handle(&self, handle: EventHandle) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.event_handles.lock().push(handle);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().push(handle);
        }
    }

    /// Hold the `handles` until the widget is deinited.
    pub fn push_event_handles(&self, handles: EventHandles) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.event_handles.lock().extend(handles);
        } else {
            WIDGET_HANDLES_CTX.get().event_handles.lock().extend(handles);
        }
    }

    /// Hold the `handle` until the widget is deinited.
    pub fn push_var_handle(&self, handle: VarHandle) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.var_handles.lock().push(handle);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().push(handle);
        }
    }

    /// Hold the `handles` until the widget is deinited.
    pub fn push_var_handles(&self, handles: VarHandles) {
        if WIDGET_HANDLES_CTX.is_default() {
            WIDGET_CTX.get().handles.var_handles.lock().extend(handles);
        } else {
            WIDGET_HANDLES_CTX.get().var_handles.lock().extend(handles);
        }
    }

    /// Widget bounds, updated every layout.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        WIDGET_CTX.get().bounds.lock().clone()
    }

    /// Widget border, updated every layout.
    pub fn border(&self) -> WidgetBorderInfo {
        WIDGET_CTX.get().border.lock().clone()
    }

    /// Gets the parent widget or `None` if is root.
    ///
    /// Panics if not called inside an widget.
    pub fn parent_id(&self) -> Option<WidgetId> {
        WIDGET_CTX.get().parent_id.load(Relaxed)
    }

    pub(crate) fn layout_is_pending(&self, layout_widgets: &LayoutUpdates) -> bool {
        let ctx = WIDGET_CTX.get();
        ctx.flags.load(Relaxed).contains(UpdateFlags::LAYOUT) || layout_widgets.delivery_list.enter_widget(ctx.id)
    }

    /// Remove update flag and returns if it intersected.
    pub(crate) fn take_update(&self, flag: UpdateFlags) -> bool {
        let mut r = false;
        let _ = WIDGET_CTX.get().flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if f.intersects(flag) {
                r = true;
                f.remove(flag);
                Some(f)
            } else {
                None
            }
        });
        r
    }

    /// Current pending updates.
    #[cfg(debug_assertions)]
    pub(crate) fn pending_update(&self) -> UpdateFlags {
        WIDGET_CTX.get().flags.load(Relaxed)
    }

    /// Remove the render reuse range if render was not invalidated on this widget.
    pub(crate) fn take_render_reuse(&self, render_widgets: &RenderUpdates, render_update_widgets: &RenderUpdates) -> Option<ReuseRange> {
        let ctx = WIDGET_CTX.get();
        let mut try_reuse = true;

        // take RENDER, RENDER_UPDATE
        let _ = ctx.flags.fetch_update(Relaxed, Relaxed, |mut f| {
            if f.intersects(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE) {
                try_reuse = false;
                f.remove(UpdateFlags::RENDER | UpdateFlags::RENDER_UPDATE);
                Some(f)
            } else {
                None
            }
        });

        if try_reuse && !render_widgets.delivery_list.enter_widget(ctx.id) && !render_update_widgets.delivery_list.enter_widget(ctx.id) {
            ctx.render_reuse.lock().take()
        } else {
            None
        }
    }

    pub(crate) fn set_render_reuse(&self, range: Option<ReuseRange>) {
        *WIDGET_CTX.get().render_reuse.lock() = range;
    }
}

context_local! {
    static WIDGET_CTX: WidgetCtxData = WidgetCtxData::no_context();
    static WIDGET_HANDLES_CTX: WidgetHandlesCtxData = WidgetHandlesCtxData::dummy();
}

/// Defines the backing data of [`WIDGET`].
///
/// Each widget owns this data and calls [`WIDGET.with_context`] to delegate to it's child node.
///
/// [`WIDGET.with_context`]: WIDGET::with_context
pub struct WidgetCtx(Option<Arc<WidgetCtxData>>);
impl WidgetCtx {
    /// New widget context.
    pub fn new(id: WidgetId) -> Self {
        Self(Some(Arc::new(WidgetCtxData {
            parent_id: Atomic::new(None),
            id,
            flags: Atomic::new(UpdateFlags::empty()),
            state: RwLock::new(OwnedStateMap::default()),
            handles: WidgetHandlesCtxData::dummy(),
            bounds: Mutex::new(WidgetBoundsInfo::default()),
            border: Mutex::new(WidgetBorderInfo::default()),
            render_reuse: Mutex::new(None),
        })))
    }

    #[cfg(test)]
    pub(crate) fn new_test(id: WidgetId, bounds: WidgetBoundsInfo, border: WidgetBorderInfo) -> Self {
        let ctx = Self::new(id);
        let c = ctx.0.as_ref().unwrap();
        *c.bounds.lock() = bounds;
        *c.border.lock() = border;
        ctx
    }

    /// Drops all var and event handles, clears all state.
    ///
    /// If `retain_state` is enabled the state will not be cleared and can still read.
    pub fn deinit(&mut self, retain_state: bool) {
        let ctx = self.0.as_mut().unwrap();
        ctx.handles.var_handles.lock().clear();
        ctx.handles.event_handles.lock().clear();
        ctx.flags.store(UpdateFlags::empty(), Relaxed);
        *ctx.render_reuse.lock() = None;

        if !retain_state {
            ctx.state.write().clear();
        }
    }

    /// Returns `true` if reinit was requested for the widget.
    ///
    /// Note that widget implementers must use [`take_reinit`] to fulfill the request.
    ///
    /// [`take_reinit`]: Self::take_reinit
    pub fn is_pending_reinit(&self) -> bool {
        self.0.as_ref().unwrap().flags.load(Relaxed).contains(UpdateFlags::REINIT)
    }

    /// Returns `true` if an [`WIDGET.reinit`] request was made.
    ///
    /// Unlike other requests, the widget implement must re-init immediately.
    ///
    /// [`WIDGET.reinit`]: WIDGET::reinit
    pub fn take_reinit(&mut self) -> bool {
        let ctx = self.0.as_mut().unwrap();

        let mut flags = ctx.flags.load(Relaxed);
        let r = flags.contains(UpdateFlags::REINIT);
        if r {
            flags.remove(UpdateFlags::REINIT);
            ctx.flags.store(flags, Relaxed);
        }

        r
    }

    /// Gets the widget id.
    pub fn id(&self) -> WidgetId {
        self.0.as_ref().unwrap().id
    }
    /// Gets the widget bounds.
    pub fn bounds(&self) -> WidgetBoundsInfo {
        self.0.as_ref().unwrap().bounds.lock().clone()
    }

    /// Gets the widget borders.
    pub fn border(&self) -> WidgetBorderInfo {
        self.0.as_ref().unwrap().border.lock().clone()
    }

    /// Call `f` with an exclusive lock to the widget state.
    pub fn with_state<R>(&mut self, f: impl FnOnce(&mut OwnedStateMap<WIDGET>) -> R) -> R {
        f(&mut self.0.as_mut().unwrap().state.write())
    }
}