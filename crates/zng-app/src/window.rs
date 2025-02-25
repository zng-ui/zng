//! Window context API.

use std::{borrow::Cow, fmt, sync::Arc};

use crate::{update::UpdatesTrace, widget::info::WidgetInfoTree};
use parking_lot::RwLock;
use zng_app_context::context_local;
use zng_state_map::{OwnedStateMap, StateId, StateMapMut, StateMapRef, StateValue};
use zng_txt::Txt;

zng_unique_id::unique_id_32! {
    /// Unique identifier of an open window.
    ///
    /// Can be obtained from [`WINDOW.id`] inside a window.
    ///
    /// # Name
    ///
    /// IDs are only unique for the same process.
    /// You can associate a [`name`] with an ID to give it a persistent identifier.
    ///
    /// [`WINDOW.id`]: crate::window::WINDOW::id
    /// [`name`]: WindowId::name
    pub struct WindowId;
}
zng_unique_id::impl_unique_id_name!(WindowId);
zng_unique_id::impl_unique_id_fmt!(WindowId);
zng_unique_id::impl_unique_id_bytemuck!(WindowId);

zng_var::impl_from_and_into_var! {
    /// Calls [`WindowId::named`].
    fn from(name: &'static str) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: String) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: Cow<'static, str>) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: char) -> WindowId {
        WindowId::named(name)
    }
    /// Calls [`WindowId::named`].
    fn from(name: Txt) -> WindowId {
        WindowId::named(name)
    }

    fn from(some: WindowId) -> Option<WindowId>;
}
impl serde::Serialize for WindowId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let name = self.name();
        if name.is_empty() {
            use serde::ser::Error;
            return Err(S::Error::custom("cannot serialize unnamed `WindowId`"));
        }
        name.serialize(serializer)
    }
}
impl<'de> serde::Deserialize<'de> for WindowId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let name = Txt::deserialize(deserializer)?;
        Ok(WindowId::named(name))
    }
}

zng_unique_id::unique_id_32! {
    /// Unique identifier of a monitor screen.
    pub struct MonitorId;
}
zng_unique_id::impl_unique_id_bytemuck!(MonitorId);
impl fmt::Debug for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            f.debug_struct("MonitorId")
                .field("id", &self.get())
                .field("sequential", &self.sequential())
                .finish()
        } else {
            write!(f, "MonitorId({})", self.sequential())
        }
    }
}
impl fmt::Display for MonitorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "MonitorId({})", self.sequential())
    }
}
impl MonitorId {
    /// ID of a fake monitor for cases where no monitor is available.
    pub fn fallback() -> MonitorId {
        static FALLBACK: once_cell::sync::Lazy<MonitorId> = once_cell::sync::Lazy::new(MonitorId::new_unique);
        *FALLBACK
    }
}

/// Current context window.
///
/// This represents the minimum features required for a window context, see `WINDOW_Ext` for more
/// features provided by the default window implementation.
///
/// # Panics
///
/// Most of the methods on this service panic if not called inside a window context.
pub struct WINDOW;
impl WINDOW {
    /// Returns `true` if called inside a window.
    pub fn is_in_window(&self) -> bool {
        !WINDOW_CTX.is_default()
    }

    /// Gets the window ID, if called inside a window.
    pub fn try_id(&self) -> Option<WindowId> {
        if WINDOW_CTX.is_default() { None } else { Some(WINDOW_CTX.get().id) }
    }

    /// Gets the window ID.
    pub fn id(&self) -> WindowId {
        WINDOW_CTX.get().id
    }

    /// Gets the window mode.
    pub fn mode(&self) -> WindowMode {
        WINDOW_CTX.get().mode
    }

    /// Gets the window info tree.
    ///
    /// Panics if called before the window future yields the window.
    pub fn info(&self) -> WidgetInfoTree {
        WINDOW_CTX.get().widget_tree.read().clone().expect("window not init")
    }

    /// Calls `f` with a read lock on the current window state map.
    pub fn with_state<R>(&self, f: impl FnOnce(StateMapRef<WINDOW>) -> R) -> R {
        f(WINDOW_CTX.get().state.read().borrow())
    }

    /// Calls `f` with a write lock on the current window state map.
    pub fn with_state_mut<R>(&self, f: impl FnOnce(StateMapMut<WINDOW>) -> R) -> R {
        f(WINDOW_CTX.get().state.write().borrow_mut())
    }

    /// Get the window state `id`, if it is set.
    pub fn get_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> Option<T> {
        let id = id.into();
        self.with_state(|s| s.get_clone(id))
    }

    /// Require the window state `id`.
    ///
    /// Panics if the `id` is not set.
    pub fn req_state<T: StateValue + Clone>(&self, id: impl Into<StateId<T>>) -> T {
        let id = id.into();
        self.with_state(|s| s.req(id).clone())
    }

    /// Set the window state `id` to `value`.
    ///
    /// Returns the previous set value.
    pub fn set_state<T: StateValue>(&self, id: impl Into<StateId<T>>, value: impl Into<T>) -> Option<T> {
        let id = id.into();
        let value = value.into();
        self.with_state_mut(|mut s| s.set(id, value))
    }

    /// Sets the window state `id` without value.
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

    /// Returns `true` if the `id` is set or flagged in the window.
    pub fn contains_state<T: StateValue>(&self, id: impl Into<StateId<T>>) -> bool {
        let id = id.into();
        self.with_state(|s| s.contains(id))
    }

    /// Calls `f` while the window is set to `ctx`.
    pub fn with_context<R>(&self, ctx: &mut WindowCtx, f: impl FnOnce() -> R) -> R {
        let _span = match ctx.0.as_mut() {
            Some(c) => UpdatesTrace::window_span(c.id),
            None => panic!("window is required"),
        };
        WINDOW_CTX.with_context(&mut ctx.0, f)
    }

    /// Calls `f` while no window is available in the context.
    pub fn with_no_context<R>(&self, f: impl FnOnce() -> R) -> R {
        WINDOW_CTX.with_default(f)
    }
}

/// Test only methods.
#[cfg(any(test, doc, feature = "test_util"))]
mod _impl {
    use zng_color::colors;
    use zng_layout::{
        context::{InlineConstraints, InlineConstraintsLayout, InlineConstraintsMeasure, LAYOUT, LayoutMetrics},
        unit::{FactorUnits, Length, Px, PxConstraints2d, PxSize, PxTransform},
    };
    use zng_state_map::{StateId, static_id};
    use zng_view_api::config::FontAntiAliasing;

    use super::*;
    use crate::{
        render::FrameValueKey,
        update::{ContextUpdates, EventUpdate, LayoutUpdates, UPDATES, UpdateDeliveryList, WidgetUpdates},
        widget::{
            WIDGET, WIDGET_CTX, WidgetCtx, WidgetId, WidgetUpdateMode,
            info::{WidgetBorderInfo, WidgetBoundsInfo, WidgetPath},
            node::UiNode,
        },
    };
    use atomic::Ordering::Relaxed;

    static_id! {
        static ref TEST_WINDOW_CFG: StateId<TestWindowCfg>;
    }

    struct TestWindowCfg {
        size: PxSize,
    }

    /// Window test helpers.
    ///
    /// # Panics
    ///
    /// Most of the test methods panic if not called inside [`with_test_context`].
    ///
    /// [`with_test_context`]: WINDOW::with_test_context
    impl WINDOW {
        /// Calls `f` inside a new headless window and root widget.
        pub fn with_test_context<R>(&self, update_mode: WidgetUpdateMode, f: impl FnOnce() -> R) -> R {
            let window_id = WindowId::new_unique();
            let root_id = WidgetId::new_unique();
            let mut ctx = WindowCtx::new(window_id, WindowMode::Headless);
            ctx.set_widget_tree(WidgetInfoTree::wgt(window_id, root_id));
            WINDOW.with_context(&mut ctx, || {
                WINDOW.set_state(
                    *TEST_WINDOW_CFG,
                    TestWindowCfg {
                        size: PxSize::new(Px(1132), Px(956)),
                    },
                );

                let mut ctx = WidgetCtx::new(root_id);
                WIDGET.with_context(&mut ctx, update_mode, f)
            })
        }

        /// Get the test window size.
        ///
        /// This size is used by the `test_*` methods that need a window size.
        pub fn test_window_size(&self) -> PxSize {
            WINDOW.with_state_mut(|mut s| s.get_mut(*TEST_WINDOW_CFG).expect("not in test window").size)
        }

        /// Set test window `size`.
        pub fn set_test_window_size(&self, size: PxSize) {
            WINDOW.with_state_mut(|mut s| {
                s.get_mut(*TEST_WINDOW_CFG).expect("not in test window").size = size;
            })
        }

        /// Call inside [`with_test_context`] to init the `content` as a child of the test window root.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_init(&self, content: &mut impl UiNode) -> ContextUpdates {
            content.init();
            WIDGET.test_root_updates();
            UPDATES.apply()
        }

        /// Call inside [`with_test_context`] to deinit the `content` as a child of the test window root.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_deinit(&self, content: &mut impl UiNode) -> ContextUpdates {
            content.deinit();
            WIDGET.test_root_updates();
            UPDATES.apply()
        }

        /// Call inside [`with_test_context`] to rebuild info the `content` as a child of the test window root.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_info(&self, content: &mut impl UiNode) -> ContextUpdates {
            let l_size = self.test_window_size();
            let mut info = crate::widget::info::WidgetInfoBuilder::new(
                Arc::default(),
                WINDOW.id(),
                crate::widget::info::access::AccessEnabled::APP,
                WIDGET.id(),
                WidgetBoundsInfo::new_size(l_size, l_size),
                WidgetBorderInfo::new(),
                1.fct(),
            );
            content.info(&mut info);
            let tree = info.finalize(Some(self.info()), false);
            *WINDOW_CTX.get().widget_tree.write() = Some(tree);
            WIDGET.test_root_updates();
            UPDATES.apply()
        }

        /// Call inside [`with_test_context`] to delivery an event to the `content` as a child of the test window root.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_event(&self, content: &mut impl UiNode, update: &mut EventUpdate) -> ContextUpdates {
            update.delivery_list_mut().fulfill_search([&WINDOW.info()].into_iter());
            content.event(update);
            WIDGET.test_root_updates();
            UPDATES.apply()
        }

        /// Call inside [`with_test_context`] to update the `content` as a child of the test window root.
        ///
        /// The `updates` can be set to a custom delivery list, otherwise window root and `content` widget are flagged for update.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_update(&self, content: &mut impl UiNode, updates: Option<&mut WidgetUpdates>) -> ContextUpdates {
            if let Some(updates) = updates {
                updates.delivery_list_mut().fulfill_search([&WINDOW.info()].into_iter());
                content.update(updates)
            } else {
                let target = if let Some(content_id) = content.with_context(WidgetUpdateMode::Ignore, || WIDGET.id()) {
                    WidgetPath::new(WINDOW.id(), vec![WIDGET.id(), content_id].into())
                } else {
                    WidgetPath::new(WINDOW.id(), vec![WIDGET.id()].into())
                };

                let mut updates = WidgetUpdates::new(UpdateDeliveryList::new_any());
                updates.delivery_list.insert_wgt(&target);

                content.update(&updates);
            }
            WIDGET.test_root_updates();
            UPDATES.apply()
        }

        /// Call inside [`with_test_context`] to layout the `content` as a child of the test window root.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_layout(&self, content: &mut impl UiNode, constraints: Option<PxConstraints2d>) -> (PxSize, ContextUpdates) {
            let font_size = Length::pt_to_px(14.0, 1.0.fct());
            let viewport = self.test_window_size();
            let mut metrics = LayoutMetrics::new(1.fct(), viewport, font_size);
            if let Some(c) = constraints {
                metrics = metrics.with_constraints(c);
            }
            let mut updates = LayoutUpdates::new(UpdateDeliveryList::new_any());
            updates.delivery_list.insert_updates_root(WINDOW.id(), WIDGET.id());
            let size = LAYOUT.with_context(metrics, || {
                crate::widget::info::WidgetLayout::with_root_widget(Arc::new(updates), |wl| content.layout(wl))
            });
            WIDGET.test_root_updates();
            (size, UPDATES.apply())
        }

        /// Call inside [`with_test_context`] to layout the `content` as a child of the test window root.
        ///
        /// Returns the measure and layout size, and the requested updates.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_layout_inline(
            &self,
            content: &mut impl UiNode,
            measure_constraints: (PxConstraints2d, InlineConstraintsMeasure),
            layout_constraints: (PxConstraints2d, InlineConstraintsLayout),
        ) -> ((PxSize, PxSize), ContextUpdates) {
            let font_size = Length::pt_to_px(14.0, 1.0.fct());
            let viewport = self.test_window_size();

            let metrics = LayoutMetrics::new(1.fct(), viewport, font_size)
                .with_constraints(measure_constraints.0)
                .with_inline_constraints(Some(InlineConstraints::Measure(measure_constraints.1)));
            let measure_size = LAYOUT.with_context(metrics, || {
                content.measure(&mut crate::widget::info::WidgetMeasure::new(Arc::default()))
            });

            let metrics = LayoutMetrics::new(1.fct(), viewport, font_size)
                .with_constraints(layout_constraints.0)
                .with_inline_constraints(Some(InlineConstraints::Layout(layout_constraints.1)));

            let mut updates = LayoutUpdates::new(UpdateDeliveryList::new_any());
            updates.delivery_list.insert_updates_root(WINDOW.id(), WIDGET.id());

            let layout_size = LAYOUT.with_context(metrics, || {
                crate::widget::info::WidgetLayout::with_root_widget(Arc::new(updates), |wl| content.layout(wl))
            });
            WIDGET.test_root_updates();
            ((measure_size, layout_size), UPDATES.apply())
        }

        /// Call inside [`with_test_context`] to render the `content` as a child of the test window root.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_render(&self, content: &mut impl UiNode) -> (crate::render::BuiltFrame, ContextUpdates) {
            use crate::render::*;

            let mut frame = {
                let win = WINDOW_CTX.get();
                let wgt = WIDGET_CTX.get();

                let frame_id = win.frame_id.load(Relaxed);
                win.frame_id.store(frame_id.next(), Relaxed);

                let f = FrameBuilder::new_renderless(
                    Arc::default(),
                    Arc::default(),
                    frame_id,
                    wgt.id,
                    &wgt.bounds.lock(),
                    win.widget_tree.read().as_ref().unwrap(),
                    1.fct(),
                    FontAntiAliasing::Default,
                );
                f
            };

            frame.push_inner(self.test_root_translation_key(), false, |frame| {
                content.render(frame);
            });

            let tree = WINDOW_CTX.get().widget_tree.read().as_ref().unwrap().clone();
            let f = frame.finalize(&tree);
            WIDGET.test_root_updates();
            (f, UPDATES.apply())
        }

        /// Call inside [`with_test_context`] to render_update the `content` as a child of the test window root.
        ///
        /// [`with_test_context`]: Self::with_test_context
        pub fn test_render_update(&self, content: &mut impl UiNode) -> (crate::render::BuiltFrameUpdate, ContextUpdates) {
            use crate::render::*;

            let mut update = {
                let win = WINDOW_CTX.get();
                let wgt = WIDGET_CTX.get();

                let frame_id = win.frame_id.load(Relaxed);
                win.frame_id.store(frame_id.next_update(), Relaxed);

                let f = FrameUpdate::new(Arc::default(), frame_id, wgt.id, wgt.bounds.lock().clone(), colors::BLACK);
                f
            };

            update.update_inner(self.test_root_translation_key(), false, |update| {
                content.render_update(update);
            });
            let tree = WINDOW_CTX.get().widget_tree.read().as_ref().unwrap().clone();
            let f = update.finalize(&tree);
            WIDGET.test_root_updates();
            (f, UPDATES.apply())
        }

        fn test_root_translation_key(&self) -> FrameValueKey<PxTransform> {
            static_id! {
                static ref ID: StateId<FrameValueKey<PxTransform>>;
            }
            WINDOW.with_state_mut(|mut s| *s.entry(*ID).or_insert_with(FrameValueKey::new_unique))
        }
    }
}

context_local! {
    static WINDOW_CTX: WindowCtxData = WindowCtxData::no_context();
}

/// Defines the backing data of [`WINDOW`].
///
/// Each window owns this data and calls [`WINDOW.with_context`](WINDOW::with_context) to delegate to it's child node.
pub struct WindowCtx(Option<Arc<WindowCtxData>>);
impl WindowCtx {
    /// New window context.
    pub fn new(id: WindowId, mode: WindowMode) -> Self {
        Self(Some(Arc::new(WindowCtxData {
            id,
            mode,
            state: RwLock::new(OwnedStateMap::default()),
            widget_tree: RwLock::new(None),

            #[cfg(any(test, doc, feature = "test_util"))]
            frame_id: atomic::Atomic::new(zng_view_api::window::FrameId::first()),
        })))
    }

    /// Sets the widget tree, must be called after every info update.
    ///
    /// Window contexts are partially available in the window new closure, but values like the `widget_tree` is
    /// available on init, so a [`WidgetInfoTree::wgt`] must be set as soon as the window and widget ID are available.
    pub fn set_widget_tree(&mut self, widget_tree: WidgetInfoTree) {
        *self.0.as_mut().unwrap().widget_tree.write() = Some(widget_tree);
    }

    /// Gets the window ID.
    pub fn id(&self) -> WindowId {
        self.0.as_ref().unwrap().id
    }

    /// Gets the window mode.
    pub fn mode(&self) -> WindowMode {
        self.0.as_ref().unwrap().mode
    }

    /// Gets the window tree.
    pub fn widget_tree(&self) -> WidgetInfoTree {
        self.0.as_ref().unwrap().widget_tree.read().as_ref().unwrap().clone()
    }

    /// Call `f` with an exclusive lock to the window state.
    pub fn with_state<R>(&mut self, f: impl FnOnce(&mut OwnedStateMap<WINDOW>) -> R) -> R {
        f(&mut self.0.as_mut().unwrap().state.write())
    }

    /// Clone a reference to the window context.
    ///
    /// This must be used only if the window implementation is split.
    pub fn share(&mut self) -> Self {
        Self(self.0.clone())
    }
}

struct WindowCtxData {
    id: WindowId,
    mode: WindowMode,
    state: RwLock<OwnedStateMap<WINDOW>>,
    widget_tree: RwLock<Option<WidgetInfoTree>>,

    #[cfg(any(test, doc, feature = "test_util"))]
    frame_id: atomic::Atomic<zng_view_api::window::FrameId>,
}
impl WindowCtxData {
    #[track_caller]
    fn no_context() -> Self {
        panic!("no window in context")
    }
}

/// Mode of an open window.
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WindowMode {
    /// Normal mode, shows a system window with content rendered.
    Headed,
    /// Headless mode, no system window and no renderer. The window does layout and calls [`UiNode::render`], but
    /// it does not actually generates frame pixels.
    ///
    /// [`UiNode::render`]: crate::widget::node::UiNode::render
    Headless,
    /// Headless mode, no visible system window but with a renderer. The window does everything a [`Headed`](WindowMode::Headed)
    /// window does, except presenting the frame in a system window.
    HeadlessWithRenderer,
}
impl fmt::Debug for WindowMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "WindowMode::")?;
        }
        match self {
            WindowMode::Headed => write!(f, "Headed"),
            WindowMode::Headless => write!(f, "Headless"),
            WindowMode::HeadlessWithRenderer => write!(f, "HeadlessWithRenderer"),
        }
    }
}
impl WindowMode {
    /// If it is the [`Headed`](WindowMode::Headed) mode.
    pub fn is_headed(self) -> bool {
        match self {
            WindowMode::Headed => true,
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => false,
        }
    }

    /// If it is the [`Headless`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::Headed) modes.
    pub fn is_headless(self) -> bool {
        match self {
            WindowMode::Headless | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headed => false,
        }
    }

    /// If it is the [`Headed`](WindowMode::Headed) or [`HeadlessWithRenderer`](WindowMode::HeadlessWithRenderer) modes.
    pub fn has_renderer(self) -> bool {
        match self {
            WindowMode::Headed | WindowMode::HeadlessWithRenderer => true,
            WindowMode::Headless => false,
        }
    }
}
