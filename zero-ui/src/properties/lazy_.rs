use std::sync::atomic::{AtomicBool, Ordering};
use std::{fmt, mem};

use crate::prelude::new_property::*;

/// Enables lazy init for the widget.
///
/// See [`LazyMode`] for details.
#[property(WIDGET, default(LazyMode::Disabled))]
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
        /// Generator for the node that replaces the widget when it is not in the init viewport.
        ///
        /// All node methods are called on the placeholder, except the render methods, it should efficiently estimate
        /// the size of the inited widget.
        placeholder: WidgetGenerator<()>,
        /// If the node is deinited when is moved out of the viewport.
        ///
        /// If `false` the node stays loaded after the first lazy load.
        deinit: bool,
    },
}
impl LazyMode {
    /// Enable lazy mode with a node that estimates the widget size.
    ///
    /// The generator must produce a node that is used as the layout placeholder for the actual widget content.
    ///
    /// The widget will init when the placeholder stops being culled by render, and deinit when it starts being culled.
    ///
    /// See [`FrameBuilder::auto_hide_rect`] for more details about render culling.
    pub fn lazy(placeholder: WidgetGenerator<()>) -> Self {
        Self::Enabled { placeholder, deinit: true }
    }

    /// Like [`lazy`] but the widget stays inited after the initial init, even if it is culled by render.
    ///
    /// [`lazy`]: Self::lazy
    pub fn once(placeholder: WidgetGenerator<()>) -> Self {
        Self::Enabled {
            placeholder,
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

        if let LazyMode::Enabled { placeholder, .. } = self.mode.get() {
            let placeholder = placeholder.generate(());
            let placeholder = crate::core::widget_base::nodes::widget_inner(placeholder).boxed();

            self.child = Some(placeholder);
        } else {
            self.child = self.not_inited.take();
        }

        self.child.init();
    }

    fn deinit(&mut self) {
        self.child.deinit();

        let node = self.child.take();
        if self.not_inited.is_none() {
            // node is actual child
            self.not_inited = node;
        }
    }

    fn update(&mut self, updates: &mut WidgetUpdates) {
        if mem::take(self.init_deinit.get_mut()) {
            if let LazyMode::Enabled { placeholder, deinit } = self.mode.get() {
                if self.not_inited.is_some() {
                    // child is placeholder, init actual child
                    self.child.deinit();
                    self.child = self.not_inited.take();
                    self.child.init();
                    WIDGET.info().layout().render();
                } else if deinit {
                    // child is actual child, deinit it, generate placeholder again.
                    self.child.deinit();
                    self.not_inited = self.child.take();

                    let placeholder = placeholder.generate(());
                    let placeholder = crate::core::widget_base::nodes::widget_inner(placeholder).boxed();

                    self.child = Some(placeholder);
                    self.child.init();
                    WIDGET.info().layout().render();
                }
            }
        } else if let Some(mode) = self.mode.get_new() {
            match mode {
                LazyMode::Enabled { .. } => {
                    // enabled, render to see if is in viewport
                    WIDGET.render();
                }
                LazyMode::Disabled => {
                    if self.not_inited.is_some() {
                        // child is placeholder, need to init actual
                        self.child.deinit();
                        self.child = self.not_inited.take();
                        self.child.init();
                        WIDGET.info().layout().render();
                    }
                }
            }
        }
        self.child.update(updates);
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if self.not_inited.is_some() {
            // child is placeholder
            let in_viewport = WIDGET.bounds().outer_bounds().intersects(&frame.auto_hide_rect());
            if in_viewport {
                self.init_deinit.store(true, Ordering::Relaxed);
                WIDGET.update();

                println!("!!: {} init, placeholder: {:?}", WIDGET.id(), WIDGET.bounds().outer_bounds());
            }
        } else {
            // child is actual child
            let deinit = self.mode.with(|l| match l {
                LazyMode::Enabled { deinit, .. } => *deinit,
                _ => false,
            });
            if deinit {
                // can deinit and this is not the first render after init.
                // we skip the first render after init to avoid flickering between
                // placeholder and actual when the placeholder does not predict
                // the actual size correctly.
                let in_viewport = WIDGET.bounds().outer_bounds().intersects(&frame.auto_hide_rect());
                if !in_viewport {
                    self.init_deinit.store(true, Ordering::Relaxed);
                    WIDGET.update();

                    println!("!!: {} deinit, actual: {:?}", WIDGET.id(), WIDGET.bounds().outer_bounds());

                    return;
                }
            }

            self.child.render(frame);
        }
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if self.not_inited.is_none() {
            // child is actual child
            self.child.render_update(update);
        }
    }
}
