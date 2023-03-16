use std::sync::atomic::{AtomicBool, Ordering};
use std::{fmt, mem};

use crate::prelude::new_property::*;
use crate::widgets::scroll::ScrollMode;

/// Enables lazy init for the widget.
///
/// See [`LazyMode`] for details.
#[property(WIDGET, default(LazyMode::Disabled))]
pub fn lazy(child: impl UiNode, mode: impl IntoVar<LazyMode>) -> impl UiNode {
    LazyNode {
        children: vec![],
        not_inited: Some(child.boxed()),
        mode: mode.into_var(),
        in_viewport: AtomicBool::new(false),
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

        /// The scroll directions that are considered for intersection with the viewport.
        ///
        /// If set to [`ScrollMode::VERTICAL`] the widget is inited if it intersects on the vertical dimension only,
        /// even if it is not actually in the viewport due to horizontal offset, and if `deinit` is flagged only the placeholder
        /// height is enforced, the width can be different from the actual.
        ///
        /// If set to [`ScrollMode::NONE`] this value is ignored, behaving like [`ScrollMode::ALL`].
        intersect: ScrollMode,
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
        Self::Enabled {
            placeholder,
            deinit: true,
            intersect: ScrollMode::ALL,
        }
    }

    /// Like [`lazy`], but only considers the height and vertical offset to init and deinit. Like [`lazy`]
    /// the placeholder height is enforced, but the width is allowed to change between placeholder and actual.
    ///
    /// Note that if the widget is inlined the full size of the widget placeholder is enforced like [`lazy`],
    /// the widget will still init and deinit considering only the vertical intersection.
    ///
    /// [`lazy`]: Self::lazy
    pub fn lazy_vertical(placeholder: WidgetGenerator<()>) -> Self {
        Self::Enabled {
            placeholder,
            deinit: true,
            intersect: ScrollMode::VERTICAL,
        }
    }

    /// Like [`lazy`], but only considers the width and horizontal offset to init and deinit. Like [`lazy`]
    /// the placeholder width is enforced, but the height is allowed to change between placeholder and actual.
    ///
    /// Note that if the widget is inlined the full size of the widget placeholder is enforced like [`lazy`],
    /// the widget will still init and deinit considering only the horizontal intersection.
    ///
    /// [`lazy`]: Self::lazy
    pub fn lazy_horizontal(placeholder: WidgetGenerator<()>) -> Self {
        Self::Enabled {
            placeholder,
            deinit: true,
            intersect: ScrollMode::HORIZONTAL,
        }
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
            intersect: ScrollMode::ALL,
        }
    }

    /// Like [`once`], but only considers the height and vertical offset to init.
    ///
    /// [`once`]: Self::once
    pub fn once_vertical(placeholder: WidgetGenerator<()>) -> Self {
        Self::Enabled {
            placeholder,
            deinit: false,
            intersect: ScrollMode::VERTICAL,
        }
    }

    /// Like [`once`], but only considers the width and horizontal offset to init.
    ///
    /// [`once`]: Self::once
    pub fn once_horizontal(placeholder: WidgetGenerator<()>) -> Self {
        Self::Enabled {
            placeholder,
            deinit: false,
            intersect: ScrollMode::HORIZONTAL,
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

    /// Unwrap and correct the intersect mode.
    fn unwrap_intersect(&self) -> ScrollMode {
        match self {
            LazyMode::Disabled => panic!("expected `LazyMode::Enabled`"),
            LazyMode::Enabled { intersect, .. } => {
                let m = *intersect;
                if m.is_empty() {
                    ScrollMode::ALL
                } else {
                    m
                }
            }
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
    children: Vec<BoxedUiNode>, // max two nodes, in `deinit` mode can be two [0]: placeholder, [1]: actual.
    not_inited: Option<BoxedUiNode>, // actual child, not inited

    #[var]
    mode: impl Var<LazyMode>,
    in_viewport: AtomicBool,
})]
impl UiNode for LazyNode {
    fn init(&mut self) {
        self.auto_subs();

        if let LazyMode::Enabled { placeholder, deinit, .. } = self.mode.get() {
            if mem::take(self.in_viewport.get_mut()) {
                // init

                if deinit {
                    // Keep placeholder, layout will still use it to avoid glitches when the actual layout causes a deinit,
                    // and the placeholder another init on a loop.
                    //
                    // This time we have the actual widget content, so the placeholder is upgraded to a full widget.

                    let placeholder = placeholder.generate(()).into_widget();
                    self.children.push(placeholder);
                }
                self.children.push(self.not_inited.take().unwrap());
            } else {
                // only placeholder

                let placeholder = placeholder.generate(());
                let placeholder = crate::core::widget_base::nodes::widget_inner(placeholder).boxed();

                // just placeholder, and as the `widget_inner`, first render may init
                self.children.push(placeholder);
            }
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

    fn measure(&self, wm: &mut WidgetMeasure) -> PxSize {
        let mut size = self.children[0].measure(wm);

        if self.not_inited.is_none() && self.children.len() == 2 {
            // is inited and can deinit, measure the actual child and validate

            let lazy_size = size;
            let actual_size = self.children[1].measure(wm);

            let mut intersect_mode = ScrollMode::empty();

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

                    intersect_mode = ScrollMode::ALL;
                } else {
                    tracing::error!("widget `{}` measure inlined, but lazy did not inline", WIDGET.id());
                }
            } else {
                if lazy_inline.is_some() {
                    tracing::error!("widget `{}` measure did not inline, but lazy did", WIDGET.id());
                }

                intersect_mode = self.mode.with(|s| s.unwrap_intersect());
            }

            if intersect_mode == ScrollMode::ALL {
                if lazy_size != actual_size {
                    tracing::error!(
                        "widget `{}` measure size `{actual_size:?}` not equal to lazy size `{lazy_size:?}`",
                        WIDGET.id()
                    );
                }
            } else if intersect_mode == ScrollMode::VERTICAL {
                if lazy_size.height != actual_size.height {
                    tracing::error!(
                        "widget `{}` measure height `{:?}` not equal to lazy height `{:?}`",
                        WIDGET.id(),
                        actual_size.height,
                        lazy_size.height,
                    );
                }

                size.width = actual_size.width;
            } else if intersect_mode == ScrollMode::HORIZONTAL {
                if lazy_size.width != actual_size.width {
                    tracing::error!(
                        "widget `{}` measure width `{:?}` not equal to lazy width `{:?}`",
                        WIDGET.id(),
                        actual_size.width,
                        lazy_size.width,
                    );
                }

                size.height = actual_size.height;
            }
        }

        size
    }

    fn layout(&mut self, wl: &mut WidgetLayout) -> PxSize {
        let mut size = UiNode::layout(&mut self.children[0], wl); // rust-analyzer errors for `.layout(wl)`

        if self.not_inited.is_none() && self.children.len() == 2 {
            // is inited and can deinit, layout the actual child and validate

            let lazy_size = size;
            let actual_size = UiNode::layout(&mut self.children[1], wl);

            let mut intersect_mode = ScrollMode::empty();

            let lazy_inlined = self.children[0].with_context(|| WIDGET.bounds().inline().is_some()).unwrap();
            if wl.inline().is_some() {
                if !lazy_inlined {
                    tracing::error!("widget `{}` inlined, but lazy did not inline", WIDGET.id());
                } else {
                    intersect_mode = ScrollMode::ALL;
                }
            } else {
                if lazy_inlined {
                    tracing::error!("widget `{}` layout did not inline, but lazy did", WIDGET.id());
                }

                intersect_mode = self.mode.with(|s| s.unwrap_intersect());
            }

            if intersect_mode == ScrollMode::ALL {
                if lazy_size != actual_size {
                    tracing::error!(
                        "widget `{}` layout size `{actual_size:?}` not equal to lazy size `{lazy_size:?}`",
                        WIDGET.id()
                    );
                }
            } else if intersect_mode == ScrollMode::VERTICAL {
                if lazy_size.height != actual_size.height {
                    tracing::error!(
                        "widget `{}` layout height `{:?}` not equal to lazy height `{:?}`",
                        WIDGET.id(),
                        actual_size.height,
                        lazy_size.height,
                    );
                }

                size.width = actual_size.width;
            } else if intersect_mode == ScrollMode::HORIZONTAL {
                if lazy_size.width != actual_size.width {
                    tracing::error!(
                        "widget `{}` layout width `{:?}` not equal to lazy width `{:?}`",
                        WIDGET.id(),
                        actual_size.width,
                        lazy_size.width,
                    );
                }

                size.height = actual_size.height;
            }
        }

        size
    }

    fn render(&self, frame: &mut FrameBuilder) {
        if self.not_inited.is_some() {
            // not inited, just verify

            let intersect_mode = self.mode.with(|s| s.unwrap_intersect());
            let outer_bounds = WIDGET.bounds().outer_bounds();
            let viewport = frame.auto_hide_rect();

            let in_viewport = if intersect_mode == ScrollMode::VERTICAL {
                outer_bounds.min_y() < viewport.max_y() && outer_bounds.max_y() > viewport.min_y()
            } else if intersect_mode == ScrollMode::HORIZONTAL {
                outer_bounds.min_x() < viewport.max_x() && outer_bounds.max_x() > viewport.min_x()
            } else {
                outer_bounds.intersects(&viewport)
            };
            if in_viewport {
                // request init
                self.in_viewport.store(true, Ordering::Relaxed);
                WIDGET.reinit();
            }
        } else if self.children.len() == 2 {
            // is inited and can deinit, check viewport on placeholder

            let intersect_mode = self.mode.with(|s| s.unwrap_intersect());
            let viewport = frame.auto_hide_rect();
            let outer_bounds = WIDGET.bounds().outer_bounds();

            let in_viewport = if intersect_mode == ScrollMode::VERTICAL {
                outer_bounds.min_y() < viewport.max_y() && outer_bounds.max_y() > viewport.min_y()
            } else if intersect_mode == ScrollMode::HORIZONTAL {
                outer_bounds.min_x() < viewport.max_x() && outer_bounds.max_x() > viewport.min_x()
            } else {
                outer_bounds.intersects(&viewport)
            };
            if !in_viewport {
                // request deinit
                self.in_viewport.store(false, Ordering::Relaxed);
                WIDGET.reinit();
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
