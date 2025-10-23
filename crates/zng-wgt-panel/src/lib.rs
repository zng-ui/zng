#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Panel widget and properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use zng_wgt::prelude::*;
use zng_wgt_wrap::Wrap;

/// Represents a dynamic layout panel.
#[widget($crate::Panel)]
pub struct Panel(WidgetBase);

impl Panel {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|wgt| {
            if let Some(p) = wgt.capture_property(property_id!(Self::children)) {
                let list = p.args.ui_node(0).clone();
                wgt.set_child(node(list, PANEL_FN_VAR));
            } else {
                wgt.set_child(node(ArcNode::new(ui_vec![]), PANEL_FN_VAR));
            }
        });
    }
}

/// Panel items.
#[property(CHILD, default(ui_vec![]), widget_impl(Panel))]
pub fn children(wgt: &mut WidgetBuilding, children: impl IntoUiNode) {
    let _ = children;
    wgt.expect_property_capture();
}

context_var! {
    /// Defines the layout widget for [`Panel!`].
    ///
    /// Is a [`Wrap!`] panel by default.
    ///
    /// [`Panel!`]: struct@Panel
    /// [`Wrap!`]: struct@Wrap
    pub static PANEL_FN_VAR: WidgetFn<PanelArgs> = wgt_fn!(|a: PanelArgs| {
        Wrap! {
            children = a.children;
        }
    });
}

/// Widget function that generates the panel layout widget.
///
/// This property can be set in any widget to affect all [`Panel!`] descendants.
///
/// This property sets [`PANEL_FN_VAR`].
///
/// [`Panel!`]: struct@Panel
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(Panel))]
pub fn panel_fn(child: impl IntoUiNode, panel: impl IntoVar<WidgetFn<PanelArgs>>) -> UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Arguments for [`panel_fn`].
///
/// [`panel_fn`]: fn@panel_fn
#[non_exhaustive]
pub struct PanelArgs {
    /// The panel children.
    ///
    /// Note that this is probably an [`ArcNode::take_on_init`] node so it may be empty until inited.
    pub children: UiNode,
}
impl PanelArgs {
    /// New args.
    pub fn new(children: UiNode) -> Self {
        Self { children }
    }
}

/// Panel widget child node.
pub fn node(children: ArcNode, panel_fn: impl IntoVar<WidgetFn<PanelArgs>>) -> UiNode {
    let panel_fn = panel_fn.into_var();
    match_node(UiNode::nil(), move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&panel_fn);
            *c.node() = panel_fn.get().call(PanelArgs {
                children: children.take_on_init(),
            });
            c.init();
        }
        UiNodeOp::Deinit => {
            c.deinit();
            *c.node() = UiNode::nil();
        }
        UiNodeOp::Update { updates } => {
            if let Some(f) = panel_fn.get_new() {
                c.deinit();
                *c.node() = f(PanelArgs {
                    children: children.take_on_init(),
                });
                c.init();
            } else {
                c.update(updates);
            }
        }
        _ => {}
    })
}
