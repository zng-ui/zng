use std::{fmt, sync::{Arc, atomic::{AtomicBool, Ordering}}};

use crate::prelude::new_property::*;

/// Enables lazy init for the widget.
///
/// See [`LazyMode`] for details.
#[property(CONTEXT, default(LazyMode::Disabled))]
pub fn lazy(child: impl UiNode, mode: impl IntoVar<LazyMode>) -> impl UiNode {
    LazyNode {
        child: None,
        not_inited: Some(child.boxed()),
        mode: mode.into_var(),
        init_deinit: AtomicBool::new(false),
    }
}

/// Lazy loading mode of an widget.
///
/// See [`lazy`] property for more details.
///
/// [`lazy`]: fn@lazy
#[derive(Clone)]
pub enum LazyMode {
    /// Node always loaded.
    Disabled,
    /// Node lazy loaded.
    Enabled {
        /// Closure called in the [`WIDGET`] and [`LAYOUT`] context to provide a size for the widget
        /// when it is not loaded.
        estimate_size: Arc<dyn Fn() -> PxSize + Send + Sync>,
        /// If the node is deinited when is moved out of the two pages from the viewport.
        ///
        /// If `false` the node stays loaded after the first lazy load.
        deinit: bool,
    },
}
impl LazyMode {
    /// Enable lazy mode with a closure that estimates the node size when it is not loaded.
    ///
    /// The closure is called in the [`WIDGET`] and [`LAYOUT`] context to provide a size for the widget
    /// when it is not loaded.
    ///
    /// The widget will init when it enters the viewport range, and deinit when it leaves it.
    pub fn lazy(estimate_size: impl Fn() -> PxSize + Send + Sync + 'static) -> Self {
        Self::Enabled {
            estimate_size: Arc::new(estimate_size),
            deinit: true,
        }
    }

    /// Like [`lazy`] but the widget stays inited after the initial init, even if it is moved out of the viewport range.
    ///
    /// [`lazy`]: Self::lazy
    pub fn once(estimate_size: impl Fn() -> PxSize + Send + Sync + 'static) -> Self {
        Self::Enabled {
            estimate_size: Arc::new(estimate_size),
            deinit: false,
        }
    }

    /// If lazy init is mode.
    pub fn is_enabled(&self) -> bool {
        matches!(self, Self::Enabled { .. })
    }

    /// If lazy init is disabled.
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }
}
impl fmt::Debug for LazyMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disabled => write!(f, "Disabled"),
            Self::Enabled { deinit, .. } => f.debug_struct("Enabled").field("deinit", deinit).finish_non_exhaustive(),
        }
    }
}

#[ui_node(struct LazyNode {
    child: Option<BoxedUiNode>,
    not_inited: Option<BoxedUiNode>,
    
    #[var]
    mode: impl Var<LazyMode>,
    init_deinit: AtomicBool,
})]
impl UiNode for LazyNode {
    fn init(&mut self) {
        self.auto_subs();
        if !self.mode.with(|e| e.is_enabled()) {
            self.child = self.not_inited.take();
        }

        self.child.init();
    }

    fn deinit(&mut self) {
        self.child.deinit();
        self.not_inited = self.child.take();
    }

    fn update(&mut self, updates: &mut WidgetUpdates) {
        if self.init_deinit.swap(false, Ordering::Relaxed) {
            if self.child.is_none() {
                // init
                self.child = self.not_inited.take();
                self.child.init();
            } else {
                // deinit
                self.child.deinit();
                self.not_inited = self.child.take();
            }
        }
        self.child.update(updates);
    }

    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        if let Some(c) = &self.child {
            c.measure(wm)
        } else {
            self.mode.with(|l| match l {
                LazyMode::Enabled { estimate_size, .. } => estimate_size(),
                _ => unreachable!()
            })
        }
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        if let Some(c) = &mut self.child {
            c.layout(wl)
        } else {
            self.mode.with(|l| match l {
                LazyMode::Enabled { estimate_size, .. } => estimate_size(),
                _ => unreachable!()
            })
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        let in_viewport = WIDGET.bounds().outer_bounds().intersects(&frame.auto_hide_rect());
        if in_viewport {
            if self.child.is_none() {
                self.init_deinit.store(true, Ordering::Relaxed);
                WIDGET.update();
            } else {
                self.child.render(frame);
            }
        } else if self.child.is_some() {
            let deinit = self.mode.with(|l| match l {
                LazyMode::Enabled { deinit, .. } => *deinit,
                _ => false
            });
            if deinit {
                self.init_deinit.store(true, Ordering::Relaxed);
                WIDGET.update();
            } else {
                self.child.render(frame);
            }
        }

    }
}
