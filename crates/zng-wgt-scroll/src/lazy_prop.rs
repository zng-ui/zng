use std::{fmt, mem};

use crate::ScrollMode;
use zng_wgt::prelude::*;

/// Lazy init mode of a widget.
///
/// See [`lazy`] property for more details.
///
/// [`lazy`]: fn@lazy
#[derive(Clone, PartialEq)]
pub enum LazyMode {
    /// Node always inited.
    Disabled,
    /// Node lazy inited.
    Enabled {
        /// Function that instantiates the node that replaces the widget when it is not in the init viewport.
        ///
        /// All node methods are called on the placeholder, except the render methods, it should efficiently estimate
        /// the size of the inited widget.
        placeholder: WidgetFn<()>,
        /// If the node is deinited when is moved out of the viewport.
        ///
        /// If `false` the node stays inited after the first lazy init.
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
        /// If set to [`ScrollMode::NONE`] this value is ignored, behaving like [`ScrollMode::PAN`].
        intersect: ScrollMode,
    },
}
impl LazyMode {
    /// Enable lazy mode with a node that estimates the widget size.
    ///
    /// The widget function must produce a node that is used as the layout placeholder for the actual widget content.
    ///
    /// The widget will init when the placeholder stops being culled by render, and deinit when it starts being culled.
    /// Note that in this mode the placeholder size is always used as the widget size, see the `deinit` docs in [`LazyMode::Enabled`]
    /// for more details.
    ///
    /// See [`FrameBuilder::auto_hide_rect`] for more details about render culling.
    ///
    /// [`FrameBuilder::auto_hide_rect`]: zng_wgt::prelude::FrameBuilder::auto_hide_rect
    pub fn lazy(placeholder: WidgetFn<()>) -> Self {
        Self::Enabled {
            placeholder,
            deinit: true,
            intersect: ScrollMode::PAN,
        }
    }

    /// Like [`lazy`], but only considers the height and vertical offset to init and deinit. Like [`lazy`]
    /// the placeholder height is enforced, but the width is allowed to change between placeholder and actual.
    ///
    /// Note that if the widget is inlined the full size of the widget placeholder is enforced like [`lazy`],
    /// the widget will still init and deinit considering only the vertical intersection.
    ///
    /// [`lazy`]: Self::lazy
    pub fn lazy_vertical(placeholder: WidgetFn<()>) -> Self {
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
    pub fn lazy_horizontal(placeholder: WidgetFn<()>) -> Self {
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
    pub fn once(placeholder: WidgetFn<()>) -> Self {
        Self::Enabled {
            placeholder,
            deinit: false,
            intersect: ScrollMode::PAN,
        }
    }

    /// Like [`once`], but only considers the height and vertical offset to init.
    ///
    /// [`once`]: Self::once
    pub fn once_vertical(placeholder: WidgetFn<()>) -> Self {
        Self::Enabled {
            placeholder,
            deinit: false,
            intersect: ScrollMode::VERTICAL,
        }
    }

    /// Like [`once`], but only considers the width and horizontal offset to init.
    ///
    /// [`once`]: Self::once
    pub fn once_horizontal(placeholder: WidgetFn<()>) -> Self {
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
                if m.is_empty() { ScrollMode::PAN } else { m }
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

/// Enables lazy init for the widget.
///
/// See [`LazyMode`] for details.
#[property(WIDGET, default(LazyMode::Disabled))]
pub fn lazy(child: impl IntoUiNode, mode: impl IntoVar<LazyMode>) -> UiNode {
    let mode = mode.into_var();
    // max two nodes:
    // * in `deinit` mode can be two [0]: placeholder, [1]: actual.
    // * or can be only placeholder or only actual.
    let children = vec![];
    // actual child, not inited
    let mut not_inited = Some(child.boxed());
    let mut in_viewport = false;

    match_node_list(children, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&mode);

            if let LazyMode::Enabled { placeholder, deinit, .. } = mode.get() {
                if mem::take(&mut in_viewport) {
                    // init

                    if deinit {
                        // Keep placeholder, layout will still use it to avoid glitches when the actual layout causes a deinit,
                        // and the placeholder another init on a loop.
                        //
                        // This time we have the actual widget content, so the placeholder is upgraded to a full widget to
                        // have a place to store the layout info.

                        let placeholder = placeholder(()).into_widget();
                        c.children().push(placeholder);
                    }
                    c.children().push(not_inited.take().unwrap());
                } else {
                    // only placeholder

                    let placeholder = placeholder(());
                    let placeholder = zng_app::widget::base::node::widget_inner(placeholder).boxed();

                    // just placeholder, and as the `widget_inner`, first render may init
                    c.children().push(placeholder);
                }
            } else {
                // not enabled, just init actual
                c.children().push(not_inited.take().unwrap());
            }
        }
        UiNodeOp::Deinit => {
            c.deinit_all();

            if not_inited.is_none() {
                not_inited = c.children().pop(); // pop actual
            }
            c.children().clear(); // drop placeholder, if any
        }
        UiNodeOp::Update { .. } => {
            if mode.is_new() {
                WIDGET.reinit();
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let mut size = c.with_node(0, |n| n.measure(wm));

            if not_inited.is_none() && c.len() == 2 {
                // is inited and can deinit, measure the actual child and validate

                let lazy_size = size;
                let actual_size = c.with_node(1, |n| n.measure(wm));

                let mut intersect_mode = ScrollMode::empty();

                let lazy_inline = c.children()[0]
                    .with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().measure_inline())
                    .flatten();
                if let Some(actual_inline) = wm.inline() {
                    if let Some(lazy_inline) = lazy_inline {
                        fn validate<T: PartialEq + fmt::Debug>(actual: T, lazy: T, name: &'static str) {
                            if actual != lazy {
                                tracing::debug!(
                                    target: "lazy",
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

                        intersect_mode = ScrollMode::PAN;
                    } else {
                        tracing::debug!(target: "lazy", "widget `{}` measure inlined, but lazy did not inline", WIDGET.id());
                    }
                } else {
                    if lazy_inline.is_some() {
                        tracing::debug!(target: "lazy", "widget `{}` measure did not inline, but lazy did", WIDGET.id());
                    }

                    intersect_mode = mode.with(|s| s.unwrap_intersect());
                }

                if intersect_mode == ScrollMode::PAN {
                    if lazy_size != actual_size {
                        tracing::debug!(
                            target: "lazy",
                            "widget `{}` measure size `{actual_size:?}` not equal to lazy size `{lazy_size:?}`",
                            WIDGET.id()
                        );
                    }
                } else if intersect_mode == ScrollMode::VERTICAL {
                    if lazy_size.height != actual_size.height {
                        tracing::debug!(
                            target: "lazy",
                            "widget `{}` measure height `{:?}` not equal to lazy height `{:?}`",
                            WIDGET.id(),
                            actual_size.height,
                            lazy_size.height,
                        );
                    }

                    size.width = actual_size.width;
                } else if intersect_mode == ScrollMode::HORIZONTAL {
                    if lazy_size.width != actual_size.width {
                        tracing::debug!(
                            target: "lazy",
                            "widget `{}` measure width `{:?}` not equal to lazy width `{:?}`",
                            WIDGET.id(),
                            actual_size.width,
                            lazy_size.width,
                        );
                    }

                    size.height = actual_size.height;
                }
            }

            *desired_size = size;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let mut size = c.with_node(0, |n| n.layout(wl));

            if not_inited.is_none() && c.len() == 2 {
                // is inited and can deinit, layout the actual child and validate

                let lazy_size = size;
                let actual_size = c.with_node(1, |n| n.layout(wl));

                let mut intersect_mode = ScrollMode::empty();

                let lazy_inlined = c.children()[0]
                    .with_context(WidgetUpdateMode::Ignore, || WIDGET.bounds().inline().is_some())
                    .unwrap();
                if wl.inline().is_some() {
                    if !lazy_inlined {
                        tracing::debug!(target: "lazy", "widget `{}` inlined, but lazy did not inline", WIDGET.id());
                    } else {
                        intersect_mode = ScrollMode::PAN;
                    }
                } else {
                    if lazy_inlined {
                        tracing::debug!(target: "lazy", "widget `{}` layout did not inline, but lazy did", WIDGET.id());
                    }

                    intersect_mode = mode.with(|s| s.unwrap_intersect());
                }

                if intersect_mode == ScrollMode::PAN {
                    if lazy_size != actual_size {
                        tracing::debug!(
                            target: "lazy",
                            "widget `{}` layout size `{actual_size:?}` not equal to lazy size `{lazy_size:?}`",
                            WIDGET.id()
                        );
                    }
                } else if intersect_mode == ScrollMode::VERTICAL {
                    if lazy_size.height != actual_size.height {
                        tracing::debug!(
                            target: "lazy",
                            "widget `{}` layout height `{:?}` not equal to lazy height `{:?}`",
                            WIDGET.id(),
                            actual_size.height,
                            lazy_size.height,
                        );
                    }

                    size.width = actual_size.width;
                } else if intersect_mode == ScrollMode::HORIZONTAL {
                    if lazy_size.width != actual_size.width {
                        tracing::debug!(
                            target: "lazy",
                            "widget `{}` layout width `{:?}` not equal to lazy width `{:?}`",
                            WIDGET.id(),
                            actual_size.width,
                            lazy_size.width,
                        );
                    }

                    size.height = actual_size.height;
                }
            }

            *final_size = size;
        }
        UiNodeOp::Render { frame } => {
            c.delegated();

            if not_inited.is_some() {
                // not inited, verify

                c.children()[0].render(frame); // update bounds

                let intersect_mode = mode.with(|s| s.unwrap_intersect());
                let outer_bounds = WIDGET.bounds().outer_bounds();
                let viewport = frame.auto_hide_rect();

                in_viewport = if intersect_mode == ScrollMode::VERTICAL {
                    outer_bounds.min_y() < viewport.max_y() && outer_bounds.max_y() > viewport.min_y()
                } else if intersect_mode == ScrollMode::HORIZONTAL {
                    outer_bounds.min_x() < viewport.max_x() && outer_bounds.max_x() > viewport.min_x()
                } else {
                    outer_bounds.intersects(&viewport)
                };
                if in_viewport {
                    // request init
                    WIDGET.reinit();
                }
            } else if c.len() == 2 {
                // is inited and can deinit, check viewport on placeholder

                c.children()[1].render(frame); // render + update bounds

                frame.hide(|f| {
                    f.with_hit_tests_disabled(|f| {
                        // update bounds (not used but can be inspected)
                        c.children()[0].render(f);
                    });
                });

                let intersect_mode = mode.with(|s| s.unwrap_intersect());
                let viewport = frame.auto_hide_rect();
                let outer_bounds = WIDGET.bounds().outer_bounds();

                in_viewport = if intersect_mode == ScrollMode::VERTICAL {
                    outer_bounds.min_y() < viewport.max_y() && outer_bounds.max_y() > viewport.min_y()
                } else if intersect_mode == ScrollMode::HORIZONTAL {
                    outer_bounds.min_x() < viewport.max_x() && outer_bounds.max_x() > viewport.min_x()
                } else {
                    outer_bounds.intersects(&viewport)
                };
                if !in_viewport {
                    // request deinit
                    WIDGET.reinit();
                }
            } else {
                // is inited and cannot deinit
                c.children()[0].render(frame);
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            c.delegated();
            if not_inited.is_none() {
                // child is actual child
                let last = c.children().len() - 1;

                c.children()[last].render_update(update);

                if last == 1 {
                    update.hidden(|u| {
                        // update bounds (not used but can be inspected)
                        c.children()[0].render_update(u);
                    });
                }
            } else {
                // update bounds
                c.children()[0].render_update(update);
            }
        }
        _ => {}
    })
}
