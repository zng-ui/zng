use std::sync::atomic::{AtomicBool, Ordering};
use std::{fmt, mem};

use crate::prelude::new_property::*;

/// Enables lazy init for the widget.
///
/// See [`LazyMode`] for details.
#[property(WIDGET, default(LazyMode::Disabled))]
pub fn lazy(child: impl UiNode, mode: impl IntoVar<LazyMode>) -> impl UiNode {
    LazyNode {
        children: vec![],
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
        ///
        /// If `true` the placeholder size is always used, this is to avoid the app entering a "flickering" loop
        /// when the actual bounds are different causing an immediate deinit. An error is logged if the placeholder
        /// size does not match.
        deinit: bool,
    },
}
impl LazyMode {
    /// Enable lazy mode with a node that estimates the widget size.
    ///
    /// The generator must produce a node that is used as the layout placeholder for the actual widget content.
    ///
    /// The widget will init when the placeholder stops being culled by render, and deinit when it starts being culled.
    /// Note that in this mode the placeholder size is always used as the widget size, see the `deinit` docs in [`LazyMode::Enabled`]
    /// for more details.
    ///
    /// See [`FrameBuilder::auto_hide_rect`] for more details about render culling.
    pub fn lazy(placeholder: WidgetGenerator<()>) -> Self {
        Self::Enabled { placeholder, deinit: true }
    }

    /// Like [`lazy`] but the widget stays inited after the first, even if it is culled by render it will be present in the UI tree.
    ///
    /// Note that this mode allows the actual size to be different from the placeholder size, so it can be used for items that
    /// can't estimate their own size exactly.
    ///
    /// This mode is only recommended for items that are "heavy" to init, but light after, otherwise the app will show degraded
    /// performance after many items are inited.
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
    children: Vec<BoxedUiNode>, // max two nodes, in `deinit` mode can be two [0]: placeholder, [1]: actual.
    not_inited: Option<BoxedUiNode>, // actual child, not inited

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

            // just placeholder, and as the `widget_inner`, first render may init
            self.children.push(placeholder);
        } else {
            // not enabled, just init actual
            self.children.push(self.not_inited.take().unwrap());
        }

        self.children.init_all();
    }

    fn deinit(&mut self) {
        self.children.deinit_all();
        if self.not_inited.is_none() {
            self.not_inited = self.children.pop(); // pop actual
        }
        self.children.clear(); // drop placeholder, if any
    }

    fn update(&mut self, updates: &mut WidgetUpdates) {
        if mem::take(self.init_deinit.get_mut()) {
            if let LazyMode::Enabled { placeholder, deinit } = self.mode.get() {
                if let Some(actual) = self.not_inited.take() {
                    // child is placeholder, init actual child
                    debug_assert_eq!(self.children.len(), 1);

                    self.children.deinit_all();
                    self.children.clear();

                    if deinit {
                        // Keep placeholder, layout will still use it to avoid glitches when the actual layout causes a deinit,
                        // and the placeholder another init on a loop.
                        //
                        // This time we have the actual widget content, so the placeholder is upgraded to a full widget.

                        let placeholder = placeholder.generate(()).into_widget();
                        self.children.push(placeholder);
                    }
                    self.children.push(actual);

                    self.children.init_all();

                    WIDGET.info().layout().render();
                } else if deinit {
                    // child is actual, deinit it, generate placeholder again.

                    debug_assert_eq!(self.children.len(), 2);
                    debug_assert!(self.not_inited.is_none());

                    self.children.deinit_all();
                    self.not_inited = self.children.pop();
                    self.children.clear();

                    let placeholder = placeholder.generate(());
                    let placeholder = crate::core::widget_base::nodes::widget_inner(placeholder).boxed();

                    self.children.push(placeholder);
                    self.children.init_all();

                    WIDGET.info().layout().render();
                }
            }
        } else if let Some(mode) = self.mode.get_new() {
            match mode {
                LazyMode::Enabled { deinit, placeholder } => {
                    if self.not_inited.is_some() {
                        debug_assert_eq!(self.children.len(), 1);

                        // already enabled, because is not inited,
                        // replace placeholder

                        self.children.deinit_all();
                        self.children.clear();

                        let placeholder = placeholder.generate(());
                        let placeholder = crate::core::widget_base::nodes::widget_inner(placeholder).boxed();

                        self.children.push(placeholder);

                        self.children.init_all();

                        WIDGET.info().layout().render();
                    } else if self.children.len() == 2 {
                        // already enabled and could deinit,
                        // remove or replace placeholder

                        self.children[0].deinit();

                        if deinit {
                            // continue to allow deinit, replace
                            let placeholder = placeholder.generate(());
                            self.children[0] = placeholder.into_widget();

                            self.children[0].init();
                        } else {
                            // cannot deinit anymore, remove
                            self.children.remove(0);
                        }

                        WIDGET.info().layout().render();
                    } else {
                        // already enabled and could not deinit,
                        // insert placeholder

                        if deinit {
                            let placeholder = placeholder.generate(());
                            self.children.insert(0, placeholder.into_widget());

                            self.children[0].init();

                            WIDGET.info().layout().render();
                        }
                    }
                }
                LazyMode::Disabled => {
                    if let Some(actual) = self.not_inited.take() {
                        // child is placeholder, need to init actual
                        self.children.deinit_all();
                        self.children.clear();

                        self.children.push(actual);

                        self.children.init_all();

                        WIDGET.info().layout().render();
                    }
                }
            }
        }
        self.children.update_all(updates, &mut ());
    }

    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        let size = self.children[0].measure(wm);

        if self.not_inited.is_none() && self.children.len() == 2 {
            // is inited and can deinit, measure the actual child and validate

            let lazy_size = size;
            let actual_size = self.children[1].measure(wm);

            if lazy_size != actual_size {
                tracing::error!(
                    "widget `{}` measure size `{actual_size:?}` not equal to lazy size `{lazy_size:?}`",
                    WIDGET.id()
                );
            }

            let lazy_inline = self.children[0].with_context(|| WIDGET.bounds().measure_inline()).flatten();

            if let Some(actual_inline) = wm.inline() {
                if let Some(lazy_inline) = lazy_inline {
                    fn validate<T: PartialEq + fmt::Debug>(actual: T, lazy: T, name: &'static str) {
                        if actual != lazy {
                            tracing::error!(
                                "widget `{}` measure inline {name} `{actual:?}` not equal to lazy `{lazy:?}`",
                                WIDGET.id()
                            );
                        }
                    }
                    validate(actual_inline.first, lazy_inline.first, "first");
                    validate(actual_inline.first_wrapped, lazy_inline.first_wrapped, "first_wrapped");
                    validate(actual_inline.last, lazy_inline.last, "last");
                    validate(actual_inline.last_wrapped, lazy_inline.last_wrapped, "last_wrapped");

                    actual_inline.first = lazy_inline.first;
                    actual_inline.first_wrapped = lazy_inline.first_wrapped;
                    actual_inline.last = lazy_inline.last;
                    actual_inline.last_wrapped = lazy_inline.last_wrapped;
                } else {
                    tracing::error!("widget `{}` measure inlined, but lazy did not inline", WIDGET.id());
                }
            } else if lazy_inline.is_some() {
                tracing::error!("widget `{}` measure did not inline, but lazy did", WIDGET.id());
            }
        }

        size
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let size = self.children[0].layout(wl);

        if self.not_inited.is_none() && self.children.len() == 2 {
            // is inited and can deinit, layout the actual child and validate

            let lazy_size = size;
            let actual_size = self.children[1].layout(wl);

            if lazy_size != actual_size {
                tracing::error!(
                    "widget `{}` layout size `{actual_size:?}` not equal to lazy size `{lazy_size:?}`",
                    WIDGET.id()
                );
            }

            let lazy_inlined = self.children[0].with_context(|| WIDGET.bounds().inline().is_some()).unwrap();
            if wl.inline().is_some() {
                if !lazy_inlined {
                    tracing::error!("widget `{}` inlined, but lazy did not inline", WIDGET.id());
                }
            } else if lazy_inlined {
                tracing::error!("widget `{}` layout did not inline, but lazy did", WIDGET.id());
            }
        }

        size
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if self.not_inited.is_some() {
            // not inited, just verify
            let in_viewport = WIDGET.bounds().outer_bounds().intersects(&frame.auto_hide_rect());
            if in_viewport {
                // request init
                self.init_deinit.store(true, Ordering::Relaxed);
                WIDGET.update();
            }
        } else if self.children.len() == 2 {
            // is inited and can deinit, check viewport on placeholder
            let placeholder_bounds = self.children[0].with_context(|| WIDGET.bounds()).unwrap();
            let in_viewport = placeholder_bounds.outer_bounds().intersects(&frame.auto_hide_rect());

            if !in_viewport {
                // request deinit
                self.init_deinit.store(true, Ordering::Relaxed);
                WIDGET.update();
            } else {
                self.children[1].render(frame);
            }
        } else {
            // is inited and cannot deinit
            self.children[0].render(frame);
        }
    }

    fn render_update(&self, update: &mut FrameUpdate) {
        if self.not_inited.is_none() {
            // child is actual child
            self.children.last().unwrap().render_update(update);
        }
    }
}
