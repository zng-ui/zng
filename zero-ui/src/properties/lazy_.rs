use std::{fmt, sync::Arc};

use crate::prelude::new_property::*;

/// Enables lazy init for the widget.
///
/// See [`LazyMode`] for details.
#[property(CONTEXT, default(LazyMode::Disabled))]
pub fn lazy(child: impl UiNode, enabled: impl IntoVar<LazyMode>) -> impl UiNode {
    LazyNode {
        child,
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
        /// when it is not loaded. The closure input is the last actual size observed if the widget was
        /// loaded at a previous time.
        estimate_size: Arc<dyn Fn(Option<PxSize>) -> PxSize + Send + Sync>,
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
    /// when it is not loaded. The closure input is the last actual size observed if the widget was
    /// loaded at a previous time.
    ///
    /// The widget will init when it enters the viewport range, and deinit when it leaves it.
    pub fn lazy(estimate_size: impl Fn(Option<PxSize>) -> PxSize + Send + Sync + 'static) -> Self {
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
            estimate_size: Arc::new(move |_| estimate_size()),
            deinit: false,
        }
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
    child: impl UiNode,
    #[var]
    enabled: impl Var<LazyMode>,
})]
impl UiNode for LazyNode {
    // !!: TODO
}
