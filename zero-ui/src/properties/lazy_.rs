use std::{fmt, sync::Arc};

use crate::prelude::new_property::*;

/// Enables lazy init for the widget.
///
/// See [`LazyMode`] for details.
#[property(CONTEXT, default(LazyMode::Disabled))]
pub fn lazy(child: impl UiNode, enabled: impl IntoVar<LazyMode>) -> impl UiNode {
    LazyNode {
        child: None,
        not_inited: Some(child.boxed()),
        enabled: enabled.into_var(),
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

    /// If lazy init is enabled.
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
    enabled: impl Var<LazyMode>,
})]
impl UiNode for LazyNode {
    fn init(&mut self) {
        self.auto_subs();
        if !self.enabled.with(|e| e.is_enabled()) {
            self.child = self.not_inited.take();
        }

        self.child.init();
    }

    fn deinit(&mut self) {
        self.child.deinit();
        self.not_inited = self.child.take();
    }

    fn update(&mut self, updates: &mut WidgetUpdates) {
        // !!: TODO, activate
        self.child.update(updates);
    }

    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        if let Some(c) = &self.child {
            c.measure(wm)
        } else {
            self.enabled.with(|l| match l {
                LazyMode::Enabled { estimate_size, .. } => estimate_size(),
                _ => unreachable!()
            })
        }
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        if let Some(c) = &mut self.child {
            c.layout(wl)
        } else {
            self.enabled.with(|l| match l {
                LazyMode::Enabled { estimate_size, .. } => estimate_size(),
                _ => unreachable!()
            })
        }
    }

    fn render(&self, frame: &mut FrameBuilder) {
        frame.auto_hide_rect(); // !!: can't we do something like this in `layout`?
                                //     so we can call `init` immediately, otherwise we will have to generate two frames
                                //     every time the widget enters auto_hide_rect.
        self.child.render(frame);
    }
}
