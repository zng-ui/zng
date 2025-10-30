use std::fmt;

use zng_layout::context::DIRECTION_VAR;

use crate::prelude::*;

/// Margin space around the widget.
///
/// This property adds side offsets to the widget inner visual, it will be combined with the other
/// layout properties of the widget to define the inner visual position and widget size.
///
/// This property disables inline layout for the widget.
#[property(LAYOUT, default(0))]
pub fn margin(child: impl IntoUiNode, margin: impl IntoVar<SideOffsets>) -> UiNode {
    let margin = margin.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&margin);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let margin = margin.layout();
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());
            *desired_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || {
                wm.measure_block(child.node())
            });
            child.delegated();
            desired_size.width += size_increment.width;
            desired_size.height += size_increment.height;
        }
        UiNodeOp::Layout { wl, final_size } => {
            let margin = margin.layout();
            let size_increment = PxSize::new(margin.horizontal(), margin.vertical());

            *final_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || child.layout(wl));
            let mut translate = PxVector::zero();
            final_size.width += size_increment.width;
            translate.x = margin.left;
            final_size.height += size_increment.height;
            translate.y = margin.top;
            wl.translate(translate);
        }
        _ => {}
    })
}

/// Aligns the widget within the available space.
///
/// This property disables inline layout for the widget.
///
/// See [`Align`] for more details.
///
///  [`Align`]: zng_layout::unit::Align
#[property(LAYOUT, default(Align::FILL))]
pub fn align(child: impl IntoUiNode, alignment: impl IntoVar<Align>) -> UiNode {
    let alignment = alignment.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&alignment);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            let align = alignment.get();
            let child_size = LAYOUT.with_constraints(align.child_constraints(LAYOUT.constraints()), || wm.measure_block(child.node()));
            child.delegated();
            *desired_size = align.measure(child_size, LAYOUT.constraints());
        }
        UiNodeOp::Layout { wl, final_size } => {
            let align = alignment.get();
            let child_size = LAYOUT.with_constraints(align.child_constraints(LAYOUT.constraints()), || child.layout(wl));
            let (size, offset, baseline) = align.layout(child_size, LAYOUT.constraints(), LAYOUT.direction());
            wl.translate(offset);
            if baseline {
                wl.translate_baseline(true);
            }
            *final_size = size;
        }
        _ => {}
    })
}

/// If the layout direction is right-to-left.
///
/// The `state` is bound to [`DIRECTION_VAR`].
///
/// [`DIRECTION_VAR`]: zng_layout::context::DIRECTION_VAR
#[property(LAYOUT)]
pub fn is_rtl(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state(child, DIRECTION_VAR.map(|s| s.is_rtl()), state)
}

/// If the layout direction is left-to-right.
///
/// The `state` is bound to [`DIRECTION_VAR`].
///
/// [`DIRECTION_VAR`]: zng_layout::context::DIRECTION_VAR
#[property(LAYOUT)]
pub fn is_ltr(child: impl IntoUiNode, state: impl IntoVar<bool>) -> UiNode {
    bind_state(child, DIRECTION_VAR.map(|s| s.is_ltr()), state)
}

/// Inline mode explicitly selected for a widget.
#[derive(Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[zng_var::impl_property_value]
pub enum InlineMode {
    /// Widget does inline if requested by the parent widget layout and is composed only of properties that support inline.
    ///
    /// This is the default behavior.
    #[default]
    Allow,
    /// Widget always does inline.
    ///
    /// If the parent layout does not setup an inline layout environment the widget itself will. This
    /// can be used to force the inline visual, such as background clipping or any other special visual
    /// that is only enabled when the widget is inlined.
    ///
    /// Note that the widget will only inline if composed only of properties that support inline.
    Inline,
    /// Widget disables inline.
    ///
    /// If the parent widget requests inline the request does not propagate for child nodes and
    /// inline is disabled on the widget.
    Block,
}
impl fmt::Debug for InlineMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "InlineMode::")?;
        }
        match self {
            Self::Allow => write!(f, "Allow"),
            Self::Inline => write!(f, "Inline"),
            Self::Block => write!(f, "Block"),
        }
    }
}
impl_from_and_into_var! {
    fn from(inline: bool) -> InlineMode {
        if inline { InlineMode::Inline } else { InlineMode::Block }
    }
}

/// Enforce an inline mode on the widget.
///
/// See [`InlineMode`] for more details.
#[property(WIDGET, default(InlineMode::Allow))]
pub fn inline(child: impl IntoUiNode, mode: impl IntoVar<InlineMode>) -> UiNode {
    let mode = mode.into_var();
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var_layout(&mode);
        }
        UiNodeOp::Measure { wm, desired_size } => {
            *desired_size = match mode.get() {
                InlineMode::Allow => child.measure(wm),
                InlineMode::Inline => {
                    if LAYOUT.inline_constraints().is_none() {
                        // enable inline for content.
                        wm.with_inline_visual(|wm| child.measure(wm))
                    } else {
                        // already enabled by parent
                        child.measure(wm)
                    }
                }
                InlineMode::Block => {
                    // disable inline, method also disables in `WidgetMeasure`
                    child.delegated();
                    wm.measure_block(child.node())
                }
            };
        }
        UiNodeOp::Layout { wl, final_size } => {
            *final_size = match mode.get() {
                InlineMode::Allow => child.layout(wl),
                InlineMode::Inline => {
                    if LAYOUT.inline_constraints().is_none() {
                        wl.to_measure(None).with_inline_visual(|wm| child.measure(wm));
                        wl.with_inline_visual(|wl| child.layout(wl))
                    } else {
                        // already enabled by parent
                        child.layout(wl)
                    }
                }
                InlineMode::Block => {
                    if wl.inline().is_some() {
                        tracing::error!("inline enabled in `layout` when it signaled disabled in the previous `measure`");
                        child.delegated();
                        wl.layout_block(child.node())
                    } else {
                        child.layout(wl)
                    }
                }
            };
        }
        _ => {}
    })
}

context_var! {
    /// Variable that indicates the context should use mobile UI themes.
    ///
    /// This is `true` by default in Android and iOS builds. It can also be set using [`force_mobile`](fn@force_mobile).
    pub static IS_MOBILE_VAR: bool = cfg!(any(target_os = "android", target_os = "ios"));
}

/// Requests mobile UI themes in desktop builds.
///
/// This property sets the [`IS_MOBILE_VAR`].
#[property(CONTEXT, default(IS_MOBILE_VAR))]
pub fn force_mobile(child: impl IntoUiNode, is_mobile: impl IntoVar<bool>) -> UiNode {
    with_context_var(child, IS_MOBILE_VAR, is_mobile)
}

/// Gets the [`IS_MOBILE_VAR`] that indicates the window or widget should use mobile UI themes.
#[property(EVENT)]
pub fn is_mobile(child: impl IntoUiNode, is_mobile: impl IntoVar<bool>) -> UiNode {
    zng_wgt::node::bind_state(child, IS_MOBILE_VAR, is_mobile)
}
