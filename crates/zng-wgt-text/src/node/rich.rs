use std::sync::Arc;

use parking_lot::RwLock;
use zng_app::widget::node::Z_INDEX;
use zng_ext_input::focus::{FOCUS_CHANGED_EVENT, WidgetFocusInfo, WidgetInfoFocusExt};
use zng_ext_window::WINDOWS;
use zng_wgt::prelude::*;

use crate::RICH_TEXT_FOCUSED_Z_VAR;

use super::{RICH_TEXT, RichText, TEXT};

pub(crate) fn rich_text_node(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let enabled = enabled.into_var();
    let child = rich_text_component(child, "rich_text");

    let mut ctx = None;
    match_node(child, move |child, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&enabled);
            if enabled.get() && TEXT.try_rich().is_none() {
                ctx = Some(Arc::new(RwLock::new(RichText { root_id: WIDGET.id() })));

                RICH_TEXT.with_context(&mut ctx, || child.init());
            }
        }
        UiNodeOp::Update { updates } => {
            if enabled.is_new() {
                WIDGET.reinit();
            } else if ctx.is_some() {
                RICH_TEXT.with_context(&mut ctx, || child.update(updates));
            }
        }
        UiNodeOp::Deinit => {
            if ctx.is_some() {
                RICH_TEXT.with_context(&mut ctx, || child.deinit());
                ctx = None;
            }
        }
        op => {
            if ctx.is_some() {
                RICH_TEXT.with_context(&mut ctx, || child.op(op));
            }
        }
    })
}

/// An UI node that implements some behavior for rich text composition.
///
/// This node is intrinsic to the `Text!` widget and is part of the `rich_text` property. Note that the
/// actual rich text editing is implemented by the `resolve_text` and `layout_text` nodes that are intrinsic to `Text!`.
///
/// The `kind` identifies what kind of component, the value `"rich_text"` is used by the `rich_text` property, the value `"text"`
/// is used by the `Text!` widget, any other value defines a [`RichTextComponent::Leaf`] that is expected to be focusable, inlined
/// and able to handle rich text composition requests.
pub fn rich_text_component(child: impl UiNode, kind: &'static str) -> impl UiNode {
    let mut focus_within = false;
    let mut prev_index = ZIndex::DEFAULT;
    let mut index_update = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            c.init();

            if TEXT.try_rich().is_some() {
                WIDGET.sub_event(&FOCUS_CHANGED_EVENT).sub_var(&RICH_TEXT_FOCUSED_Z_VAR);
                prev_index = Z_INDEX.get();
            }
        }
        UiNodeOp::Deinit => {
            focus_within = false;
        }
        UiNodeOp::Info { info } => {
            if let Some(r) = TEXT.try_rich() {
                let c = match kind {
                    "rich_text" => {
                        if r.root_id == WIDGET.id() {
                            RichTextComponent::Root
                        } else {
                            RichTextComponent::Branch
                        }
                    }
                    kind => RichTextComponent::Leaf { kind },
                };
                info.set_meta(*RICH_TEXT_COMPONENT_ID, c);
            }
        }
        UiNodeOp::Event { update } => {
            if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                let new_is_focus_within = args.is_focus_within(WIDGET.id());
                if focus_within != new_is_focus_within {
                    focus_within = new_is_focus_within;

                    if TEXT.try_rich().is_some() {
                        index_update = Some(focus_within);
                        WIDGET.update(); // Z_INDEX.set only works on update
                    }
                }
            }
        }
        UiNodeOp::Update { updates } => {
            c.update(updates);

            if let Some(apply) = index_update.take() {
                if apply {
                    prev_index = Z_INDEX.get();
                    if let Some(i) = RICH_TEXT_FOCUSED_Z_VAR.get() {
                        Z_INDEX.set(i);
                    }
                } else if RICH_TEXT_FOCUSED_Z_VAR.get().is_some() {
                    Z_INDEX.set(prev_index);
                }
            }
            if let Some(idx) = RICH_TEXT_FOCUSED_Z_VAR.get_new() {
                if focus_within {
                    Z_INDEX.set(idx.unwrap_or(prev_index));
                }
            }
        }
        _ => {}
    })
}

impl RichText {
    /// Get root widget info.
    ///
    /// See also [`RichTextWidgetInfoExt`] to query the
    pub fn root_info(&self) -> Option<WidgetInfo> {
        WINDOWS.widget_info(self.root_id)
    }
}

/// Extends [`WidgetInfo`] state to provide information about rich text.
pub trait RichTextWidgetInfoExt {
    /// Gets the outer most ancestor that defines the rich text root.
    fn rich_text_root(&self) -> Option<WidgetInfo>;

    /// Gets what kind of component of the rich text tree this widget is.
    fn rich_text_component(&self) -> Option<RichTextComponent>;

    /// Iterate over the text/leaf component descendants that can be interacted with.
    fn rich_text_leafs(&self) -> impl Iterator<Item = WidgetFocusInfo>;
    /// Iterate over the text/leaf component descendants that can be interacted with, in reverse.
    fn rich_text_leafs_rev(&self) -> impl Iterator<Item = WidgetFocusInfo>;

    /// Iterate over the prev text/leaf components before the current one.
    fn rich_text_prev(&self) -> impl Iterator<Item = WidgetFocusInfo>;

    /// Iterate over the next text/leaf components after the current one.
    fn rich_text_next(&self) -> impl Iterator<Item = WidgetFocusInfo>;
}
impl RichTextWidgetInfoExt for WidgetInfo {
    fn rich_text_root(&self) -> Option<WidgetInfo> {
        self.self_and_ancestors()
            .find(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Root)))
    }

    fn rich_text_component(&self) -> Option<RichTextComponent> {
        self.meta().copy(*RICH_TEXT_COMPONENT_ID)
    }

    fn rich_text_leafs(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        self.descendants()
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
            .filter_map(|w| w.into_focusable(false, false))
    }
    fn rich_text_leafs_rev(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        self.descendants()
            .tree_rev()
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
            .filter_map(|w| w.into_focusable(false, false))
    }

    fn rich_text_prev(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        self.rich_text_root()
            .into_iter()
            .flat_map(|w| self.prev_siblings_in(&w))
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
            .filter_map(|w| w.into_focusable(false, false))
    }

    fn rich_text_next(&self) -> impl Iterator<Item = WidgetFocusInfo> {
        self.rich_text_root()
            .into_iter()
            .flat_map(|w| self.next_siblings_in(&w))
            .filter(|w| matches!(w.rich_text_component(), Some(RichTextComponent::Leaf { .. })))
            .filter_map(|w| w.into_focusable(false, false))
    }
}

/// Represents what kind of component the widget is in a rich text tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RichTextComponent {
    /// Outermost widget that is `rich_text` enabled.
    Root,
    /// Widget is `rich_text` enabled, but is inside another rich text tree.
    Branch,
    /// Widget is a text or custom component of the rich text.
    Leaf {
        /// Arbitrary identifier.
        ///
        /// Is `"text"` for `Text!` widgets.
        kind: &'static str,
    },
}

static_id! {
    static ref RICH_TEXT_COMPONENT_ID: StateId<RichTextComponent>;
}
