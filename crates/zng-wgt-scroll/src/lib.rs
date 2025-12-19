#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Scroll widgets, properties and nodes.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_app::update::UpdatesTraceUiNodeExt as _;
use zng_wgt::{clip_to_bounds, prelude::*};

pub mod cmd;
pub mod node;
pub mod scrollbar;
pub mod thumb;

mod scroll_properties;
pub use scroll_properties::*;

mod zoom_size;
pub use zoom_size::*;

mod types;
pub use types::*;

mod lazy_prop;
pub use lazy_prop::*;

#[doc(inline)]
pub use scrollbar::Scrollbar;
#[doc(inline)]
pub use thumb::Thumb;

use zng_ext_input::focus::FocusScopeOnFocus;
use zng_wgt_container::{Container, child_align};
use zng_wgt_input::focus::{focus_scope, focus_scope_behavior};

/// A container that can pan and zoom a child of any size.
///
/// # Shorthand
///
/// The `Scroll!` macro provides shorthand syntax:
///
/// * `Scroll!($child:expr)` creates a default scroll with the child widget.
/// * `Scroll!($mode:ident, $child:expr)` Creates a scroll with one of the [`ScrollMode`] const and child widget.
/// * `Scroll!($mode:expr, $child:expr)` Creates a scroll with the [`ScrollMode`] and child widget.
#[widget($crate::Scroll {
    ($MODE:ident, $child:expr $(,)?) => {
        mode = $crate::ScrollMode::$MODE;
        child = $child;
    };
    ($mode:expr, $child:expr $(,)?) => {
        mode = $mode;
        child = $child;
    };
    ($child:expr) => {
        child = $child;
    };
})]
pub struct Scroll(ScrollUnitsMix<ScrollbarFnMix<Container>>);

/// Scroll mode.
///
/// Is [`ScrollMode::ZOOM`] by default.
#[property(CONTEXT, default(ScrollMode::ZOOM), widget_impl(Scroll))]
pub fn mode(wgt: &mut WidgetBuilding, mode: impl IntoVar<ScrollMode>) {
    let _ = mode;
    wgt.expect_property_capture();
}

impl Scroll {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            child_align = Align::CENTER;
            clip_to_bounds = true;
            focusable = true;
            focus_scope = true;
            focus_scope_behavior = FocusScopeOnFocus::LastFocused;
        }
        self.widget_builder().push_build_action(on_build);
    }

    widget_impl! {
        /// Content alignment when it is smaller then the viewport.
        ///
        /// Note that [`Align::FILL`] only applies in dimensions without scrolling.
        ///
        /// Is `CENTER` by default.
        ///
        /// [`Align::FILL`]: zng_wgt::prelude::Align::FILL
        pub child_align(align: impl IntoVar<Align>);

        /// Clip content to only be visible within the scroll bounds, including under scrollbars.
        ///
        /// Enabled by default.
        pub zng_wgt::clip_to_bounds(clip: impl IntoVar<bool>);

        /// Enables keyboard controls.
        pub zng_wgt_input::focus::focusable(focusable: impl IntoVar<bool>);

        /// Inverts priority for mouse wheel gesture so that it zooms when no modifier is pressed and
        /// scrolls when `CTRL` is pressed.
        pub zng_wgt_input::mouse::ctrl_scroll(enabled: impl IntoVar<bool>);
    }
}

/// Clip content to only be visible within the viewport, not under scrollbars.
///
/// Disabled by default.
#[property(CONTEXT, default(false), widget_impl(Scroll))]
pub fn clip_to_viewport(wgt: &mut WidgetBuilding, clip: impl IntoVar<bool>) {
    let _ = clip;
    wgt.expect_property_capture();
}

/// Properties that define scroll units.
#[widget_mixin]
pub struct ScrollUnitsMix<P>(P);

/// Properties that defines the scrollbar widget used in scrolls.
#[widget_mixin]
pub struct ScrollbarFnMix<P>(P);

fn on_build(wgt: &mut WidgetBuilding) {
    let mode = wgt.capture_var_or_else(property_id!(mode), || ScrollMode::ZOOM);

    let child_align = wgt.capture_var_or_else(property_id!(child_align), || Align::CENTER);
    let clip_to_viewport = wgt.capture_var_or_default(property_id!(clip_to_viewport));

    wgt.push_intrinsic(
        NestGroup::CHILD_CONTEXT,
        "scroll_node",
        clmv!(mode, |child| {
            let child = scroll_node(child, mode, child_align, clip_to_viewport);
            node::overscroll_node(child)
        }),
    );

    wgt.push_intrinsic(NestGroup::EVENT, "commands", |child| {
        let child = node::access_scroll_node(child);
        let child = node::scroll_to_node(child);
        let child = node::scroll_commands_node(child);
        let child = node::page_commands_node(child);
        let child = node::scroll_to_edge_commands_node(child);
        let child = node::scroll_touch_node(child);
        let child = node::zoom_commands_node(child);
        let child = node::auto_scroll_node(child);
        node::scroll_wheel_node(child)
    });

    wgt.push_intrinsic(NestGroup::CONTEXT, "context", move |child| {
        let child = with_context_var(child, SCROLL_VIEWPORT_SIZE_VAR, var(PxSize::zero()));
        let child = with_context_var(child, SCROLL_CONTENT_ORIGINAL_SIZE_VAR, var(PxSize::zero()));

        let child = with_context_var(child, SCROLL_VERTICAL_RATIO_VAR, var(0.fct()));
        let child = with_context_var(child, SCROLL_HORIZONTAL_RATIO_VAR, var(0.fct()));

        let child = with_context_var(child, SCROLL_VERTICAL_CONTENT_OVERFLOWS_VAR, var(false));
        let child = with_context_var(child, SCROLL_HORIZONTAL_CONTENT_OVERFLOWS_VAR, var(false));

        let child = SCROLL.config_node(child);

        let child = with_context_var(child, SCROLL_VERTICAL_OFFSET_VAR, var(0.fct()));
        let child = with_context_var(child, SCROLL_HORIZONTAL_OFFSET_VAR, var(0.fct()));

        let child = with_context_var(child, OVERSCROLL_VERTICAL_OFFSET_VAR, var(0.fct()));
        let child = with_context_var(child, OVERSCROLL_HORIZONTAL_OFFSET_VAR, var(0.fct()));

        let child = with_context_var(child, SCROLL_SCALE_VAR, var(1.fct()));

        with_context_var(child, SCROLL_MODE_VAR, mode)
    });
}

fn scroll_node(
    child: impl IntoUiNode,
    mode: impl IntoVar<ScrollMode>,
    child_align: impl IntoVar<Align>,
    clip_to_viewport: impl IntoVar<bool>,
) -> UiNode {
    // # Layout
    //
    // +-----------------+---+
    // |                 |   |
    // | 0 - viewport    | 1 | - v_scrollbar
    // |                 |   |
    // +-----------------+---+
    // | 2 - h_scrollbar | 3 | - scrollbar_joiner
    // +-----------------+---+
    let children = ui_vec![
        clip_to_bounds(
            node::viewport(child, mode.into_var(), child_align).instrument("viewport"),
            clip_to_viewport.into_var()
        ),
        node::v_scrollbar_presenter(),
        node::h_scrollbar_presenter(),
        node::scrollbar_joiner_presenter(),
    ];

    let scroll_info = ScrollInfo::default();

    let mut viewport = PxSize::zero();
    let mut joiner = PxSize::zero();
    let spatial_id = SpatialFrameId::new_unique();

    match_node(children, move |cs, op| match op {
        UiNodeOp::Info { info } => {
            info.set_meta(*SCROLL_INFO_ID, scroll_info.clone());
        }
        UiNodeOp::Measure { wm, desired_size } => {
            cs.delegated();
            let constraints = LAYOUT.constraints();
            *desired_size = if constraints.is_fill_max().all() {
                constraints.fill_size()
            } else {
                let size = cs.node().with_child(0, |n| n.measure(wm));
                constraints.clamp_size(size)
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            cs.delegated();
            let constraints = LAYOUT.constraints();

            // scrollbars
            let c = constraints.with_new_min(Px(0), Px(0));
            {
                joiner.width = LAYOUT.with_constraints(c.with_fill(false, true), || {
                    cs.node().with_child(1, |n| n.measure(&mut wl.to_measure(None))).width
                });
                joiner.height = LAYOUT.with_constraints(c.with_fill(true, false), || {
                    cs.node().with_child(2, |n| n.measure(&mut wl.to_measure(None))).height
                });
            }
            joiner.width = LAYOUT.with_constraints(c.with_fill(false, true).with_less_y(joiner.height), || {
                cs.node().with_child(1, |n| n.layout(wl)).width
            });
            joiner.height = LAYOUT.with_constraints(c.with_fill(true, false).with_less_x(joiner.width), || {
                cs.node().with_child(2, |n| n.layout(wl)).height
            });

            // joiner
            let _ = LAYOUT.with_constraints(PxConstraints2d::new_fill_size(joiner), || cs.node().with_child(3, |n| n.layout(wl)));

            scroll_info.set_joiner_size(joiner);

            // viewport
            let mut vp = LAYOUT.with_constraints(constraints.with_less_size(joiner), || cs.node().with_child(0, |n| n.layout(wl)));

            // collapse scrollbars if they take more the 1/3 of the total area.
            if vp.width < joiner.width * 3.0.fct() {
                vp.width += joiner.width;
                joiner.width = Px(0);
            }
            if vp.height < joiner.height * 3.0.fct() {
                vp.height += joiner.height;
                joiner.height = Px(0);
            }

            if vp != viewport {
                viewport = vp;
                WIDGET.render();
            }

            *final_size = viewport + joiner;
        }

        UiNodeOp::Render { frame } => {
            cs.delegated();

            cs.node().with_child(0, |n| n.render(frame));

            if joiner.width > Px(0) {
                let transform = PxTransform::from(PxVector::new(viewport.width, Px(0)));
                frame.push_reference_frame((spatial_id, 1).into(), FrameValue::Value(transform), true, false, |frame| {
                    cs.node().with_child(1, |n| n.render(frame));
                });
            }

            if joiner.height > Px(0) {
                let transform = PxTransform::from(PxVector::new(Px(0), viewport.height));
                frame.push_reference_frame((spatial_id, 2).into(), FrameValue::Value(transform), true, false, |frame| {
                    cs.node().with_child(2, |n| n.render(frame));
                });
            }

            if joiner.width > Px(0) && joiner.height > Px(0) {
                let transform = PxTransform::from(viewport.to_vector());
                frame.push_reference_frame((spatial_id, 3).into(), FrameValue::Value(transform), true, false, |frame| {
                    cs.node().with_child(3, |n| n.render(frame));
                });
            }
        }
        UiNodeOp::RenderUpdate { update } => {
            cs.delegated();

            cs.node().with_child(0, |n| n.render_update(update));

            if joiner.width > Px(0) {
                let transform = PxTransform::from(PxVector::new(viewport.width, Px(0)));
                update.with_transform_value(&transform, |update| {
                    cs.node().with_child(1, |n| n.render_update(update));
                });
            }

            if joiner.height > Px(0) {
                let transform = PxTransform::from(PxVector::new(Px(0), viewport.height));
                update.with_transform_value(&transform, |update| {
                    cs.node().with_child(2, |n| n.render_update(update));
                });
            }

            if joiner.width > Px(0) && joiner.height > Px(0) {
                let transform = PxTransform::from(viewport.to_vector());
                update.with_transform_value(&transform, |update| {
                    cs.node().with_child(3, |n| n.render_update(update));
                });
            }
        }
        _ => {}
    })
}
