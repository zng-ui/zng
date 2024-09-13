//! Widgets for building custom window chrome (window decorations).

use zng_ext_window::WINDOW_Ext;
use zng_wgt::prelude::*;

/// Custom window chrome.
///
/// This widget only negotiates the [`HAS_CUSTOM_CHROME`] and `chrome_overlay` instantiation. The custom chrome
/// is only instantiated if the is not already another custom chrome on the same window and the [`WindowVars::chrome`] is `false`.
///
/// Note that no chrome overlay is provided by the widget (or this crate). The main Zng crate provides a fallback custom frame.
///
/// [`WindowVars::chrome`]: zng_ext_window::WindowVars::chrome
#[widget($crate::custom_chrome::WindowChrome {
    ($root:expr) => {
        root = $root;
    }
})]
pub struct WindowChrome(WidgetBase);

impl WindowChrome {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|b| {
            let child = b.capture_ui_node(crate::property_id!(Self::root)).unwrap();
            b.set_child(child);
        });
    }

    widget_impl! {
        /// Margin around the window root, must match the window chrome overlay offsets, otherwise the
        /// window content will be under the borders and title bar.
        pub zng_wgt_container::padding(padding: impl IntoVar<SideOffsets>);
    }
}

static_id! {
    /// If set on [`WINDOW`] indicates that the an widget on the root renders a custom window chrome.
    pub static ref HAS_CUSTOM_CHROME: StateId<()>;
}

/// The window root.
#[property(CHILD, capture, widget_impl(WindowChrome))]
pub fn root(child: impl UiNode) {}

/// Overlay widget shown when the window has no system chrome.
#[property(CHILD, capture, widget_impl(WindowChrome))]
pub fn chrome_overlay(chrome: impl IntoVar<WidgetFn<()>>) {}

/// Window custom chrome implementer node.
pub fn window_chrome_node(child: BoxedUiNode, chrome_overlay: BoxedVar<WidgetFn<()>>, padding: BoxedVar<SideOffsets>) -> impl UiNode {
    let mut sys_chrome = None;
    match_node_list(ui_vec![child], move |c, op| match op {
        UiNodeOp::Init => {
            if !WINDOW.flag_state(*HAS_CUSTOM_CHROME) {
                let sys_c = WINDOW.vars().chrome();
                WIDGET.sub_var(&sys_c);
                sys_chrome = Some(sys_c);

                if sys_chrome.as_ref().map(|c| c.get()).unwrap_or(false) {
                    let mut chrome = chrome_overlay.get()(());
                    chrome.init();
                    c.children().push(chrome);
                }
            } else {
                tracing::debug!("window already has a custom window chrome node");
            }
        }
        UiNodeOp::Update { updates } => {
            if let Some(sc) = sys_chrome.as_ref() {
                if sc.is_new() || chrome_overlay.is_new() {
                    let mut changed = false;
                    for mut chrome in c.children().drain(1..) {
                        chrome.deinit();
                        changed = true;
                    }

                    if sc.get() {
                        let mut chrome = chrome_overlay.get()(());
                        chrome.init();
                        c.children().push(chrome);
                        c.children()[0].update(updates);
                        c.delegated();
                        changed = true;
                    }

                    if changed {
                        WIDGET.update_info().layout().render();
                    }
                }
            }
        }
        UiNodeOp::Measure { wm, desired_size } => {
            if sys_chrome.as_ref().map(|c| c.get()).unwrap_or(false) {
                let padding = padding.layout();
                let size_increment = PxSize::new(padding.horizontal(), padding.vertical());
                *desired_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || {
                    wm.measure_block(&mut c.children()[0])
                });
                desired_size.width += size_increment.width;
                desired_size.height += size_increment.height;
                c.delegated();
            }
        }
        UiNodeOp::Layout { wl, final_size } => {
            if sys_chrome.as_ref().map(|c| c.get()).unwrap_or(false) {
                let padding = padding.layout();
                let size_increment = PxSize::new(padding.horizontal(), padding.vertical());

                *final_size = LAYOUT.with_constraints(LAYOUT.constraints().with_less_size(size_increment), || c.children()[0].layout(wl));
                let mut translate = PxVector::zero();
                final_size.width += size_increment.width;
                translate.x = padding.left;
                final_size.height += size_increment.height;
                translate.y = padding.top;
                wl.translate(translate);
            }
        }
        _ => {}
    })
}

/// Defines an widget that resizes or moves the parent window.
#[widget($crate::custom_chrome::WindowChromeThumb)]
pub struct WindowChromeThumb(WidgetBase);
impl WindowChromeThumb {
    fn widget_intrinsic(&mut self) {
        self.widget_builder().push_build_action(|b| {
            let child = b.capture_ui_node(crate::property_id!(Self::root)).unwrap();
            b.set_child(child);
        });
    }
}

///  Defines the operation performed by this thumb.
#[property(CONTEXT, capture, widget_impl(WindowChromeThumb))]
pub fn mode(mode: impl IntoVar<ChromeThumbMode>) {}

/// Defines the operation performed by a [`WindowChromeThumb!`].
///
/// [`WindowChromeThumb!`]: struct@WindowChromeThumb
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ChromeThumbMode {
    /// Moves the entire window.
    Move,

    /// Moves the east border.
    EResize,
    /// Moves the north border.
    NResize,
    /// Moves the north-east corner.
    NeResize,
    /// Moves the north-west corner.
    NwResize,
    /// Moves the south border.
    SResize,
    /// Moves the south-east corner.
    SeResize,
    /// Moves the south-west corner.
    SwResize,
    /// Moves the west border.
    WResize,
    /// Moves the east and west borders.
    EwResize,
    /// Moves the south and north borders.
    NsResize,
}
