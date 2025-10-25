//! Sub-menu popup widget and properties.

use std::sync::Arc;

use colors::BASE_COLOR_VAR;
use zng_ext_input::{
    focus::{FOCUS, FOCUS_CHANGED_EVENT, WidgetInfoFocusExt as _},
    keyboard::{KEY_INPUT_EVENT, Key, KeyState},
};
use zng_layout::unit::Orientation2D;
use zng_wgt::{base_color, border, prelude::*};
use zng_wgt_fill::background_color;
use zng_wgt_input::pointer_capture::{CaptureMode, capture_pointer_on_init};
use zng_wgt_layer::popup::{POPUP, POPUP_CLOSE_CMD, POPUP_CLOSE_REQUESTED_EVENT, PopupCloseMode};
use zng_wgt_stack::Stack;
use zng_wgt_style::{impl_style_fn, style_fn};

use super::sub::{HOVER_OPEN_DELAY_VAR, SubMenuWidgetInfoExt};

/// Sub-menu popup.
#[widget($crate::popup::SubMenuPopup)]
pub struct SubMenuPopup(zng_wgt_layer::popup::Popup);
impl SubMenuPopup {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;

            // Supports press-and-drag to click gesture:
            //
            // - Sub-menu is `capture_pointer = true`.
            // - Menu items set`click_mode = release`.
            //
            // So the user can press to open the menu, then drag over an item and release to click it.
            capture_pointer_on_init = CaptureMode::Subtree;
            zng_wgt_rule_line::collapse_scope = true;
        }

        self.widget_builder().push_build_action(|wgt| {
            let id = wgt.capture_value::<WidgetId>(property_id!(Self::parent_id));
            let children = wgt
                .capture_property(property_id!(Self::children))
                .map(|p| p.args.ui_node(0).clone())
                .unwrap_or_else(|| ArcNode::new(ui_vec![]));

            wgt.set_child(sub_menu_popup_node(children, id));
        });
    }
}
impl_style_fn!(SubMenuPopup, DefaultStyle);

/// Sub-menu items.
#[property(CHILD, default(ui_vec![]), widget_impl(SubMenuPopup))]
pub fn children(wgt: &mut WidgetBuilding, children: impl IntoUiNode) {
    let _ = children;
    wgt.expect_property_capture();
}

/// Parent sub-menu ID.
#[property(CONTEXT, widget_impl(SubMenuPopup))]
pub fn parent_id(wgt: &mut WidgetBuilding, submenu_id: impl IntoValue<WidgetId>) {
    let _ = submenu_id;
    wgt.expect_property_capture();
}

context_var! {
    /// Defines the layout widget for [`SubMenuPopup!`].
    ///
    /// Is [`default_panel_fn`] by default.
    ///
    /// [`SubMenuPopup!`]: struct@SubMenuPopup
    pub static PANEL_FN_VAR: WidgetFn<zng_wgt_panel::PanelArgs> = WidgetFn::new(default_panel_fn);
}

/// Widget function that generates the sub-menu popup layout.
///
/// This property sets [`PANEL_FN_VAR`].
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(SubMenuPopup, DefaultStyle))]
pub fn panel_fn(child: impl IntoUiNode, panel: impl IntoVar<WidgetFn<zng_wgt_panel::PanelArgs>>) -> UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Sub-menu popup default style.
#[widget($crate::popup::DefaultStyle)]
pub struct DefaultStyle(zng_wgt_layer::popup::DefaultStyle);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            super::sub::style_fn = style_fn!(|_| super::sub::SubMenuStyle!());

            base_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));
            background_color = BASE_COLOR_VAR.rgba();
            border = {
                widths: 1,
                sides: BASE_COLOR_VAR.shade_into(1),
            };
        }
    }
}

/// Default sub-menu popup panel view.
///
/// See [`PANEL_FN_VAR`] for more details.
pub fn default_panel_fn(args: zng_wgt_panel::PanelArgs) -> UiNode {
    // remove arrow key shortcuts, they are used to navigate focus.
    let scroll_id = WidgetId::new_unique();
    zng_wgt_scroll::cmd::SCROLL_UP_CMD
        .scoped(scroll_id)
        .shortcut()
        .set(Shortcuts::new());
    zng_wgt_scroll::cmd::SCROLL_DOWN_CMD
        .scoped(scroll_id)
        .shortcut()
        .set(Shortcuts::new());

    zng_wgt_scroll::Scroll! {
        id = scroll_id;
        focusable = false;
        child_align = Align::FILL;
        child = Stack! {
            children_align = Align::FILL;
            children = args.children;
            direction = zng_wgt_stack::StackDirection::top_to_bottom();
        };
        mode = zng_wgt_scroll::ScrollMode::VERTICAL;
    }
}

/// Sub-menu popup implementation.
pub fn sub_menu_popup_node(children: ArcNode, parent: Option<WidgetId>) -> UiNode {
    let child = zng_wgt_panel::node(
        children,
        if parent.is_none() {
            super::context::PANEL_FN_VAR
        } else {
            PANEL_FN_VAR
        },
    );
    let mut close_timer = None;
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_event(&KEY_INPUT_EVENT)
                .sub_event(&POPUP_CLOSE_REQUESTED_EVENT)
                .sub_event(&FOCUS_CHANGED_EVENT);
        }
        UiNodeOp::Deinit => {
            close_timer = None;
        }
        UiNodeOp::Info { info } => {
            // sub-menus set the popup as parent in context menu.
            let parent_ctx = Some(parent.unwrap_or_else(|| WIDGET.id()));
            super::sub::SUB_MENU_PARENT_CTX.with_context(&mut Some(Arc::new(parent_ctx)), || c.info(info));
            info.set_meta(*super::sub::SUB_MENU_POPUP_ID, super::sub::SubMenuPopupInfo { parent });
        }
        UiNodeOp::Event { update } => {
            c.event(update);

            if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                if let KeyState::Pressed = args.state {
                    match &args.key {
                        Key::Escape => {
                            let info = WIDGET.info();
                            if let Some(m) = info.submenu_parent() {
                                args.propagation().stop();

                                FOCUS.focus_widget(m.id(), true);
                                POPUP.force_close_id(info.id());
                            }
                        }
                        Key::ArrowLeft | Key::ArrowRight => {
                            if let Some(info) = WINDOW.info().get(args.target.widget_id()) {
                                let info = info.into_focus_info(true, true);
                                if info.focusable_left().is_none() && info.focusable_right().is_none() {
                                    // escape to parent or change root.
                                    if let Some(m) = info.info().submenu_parent() {
                                        let mut escape = false;
                                        if m.submenu_parent().is_some()
                                            && let Some(o) = m.orientation_from(info.info().center())
                                        {
                                            escape = match o {
                                                Orientation2D::Left => args.key == Key::ArrowLeft,
                                                Orientation2D::Right => args.key == Key::ArrowRight,
                                                Orientation2D::Below | Orientation2D::Above => false,
                                            };
                                        }

                                        if escape {
                                            args.propagation().stop();
                                            // escape

                                            FOCUS.focus_widget(m.id(), true);
                                            POPUP.force_close_id(WIDGET.id());
                                        } else if let Some(m) = info.info().submenu_root() {
                                            args.propagation().stop();
                                            // change root

                                            let m = m.into_focus_info(true, true);
                                            let next_root = match &args.key {
                                                Key::ArrowLeft => m.next_left(),
                                                Key::ArrowRight => m.next_right(),
                                                _ => unreachable!(),
                                            };
                                            if let Some(n) = next_root {
                                                FOCUS.focus_widget(n.info().id(), true);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            } else if let Some(args) = POPUP_CLOSE_REQUESTED_EVENT.on_unhandled(update) {
                let sub_self = if parent.is_some() {
                    WIDGET.info().submenu_parent()
                } else {
                    // is context menu
                    Some(WIDGET.info())
                };
                if let Some(sub_self) = sub_self {
                    let mut close_ancestors = Some(None);

                    if let Some(focused) = FOCUS.focused().get()
                        && let Some(focused) = sub_self.tree().get(focused.widget_id())
                        && let Some(sub_focused) = focused.submenu_parent()
                    {
                        if sub_focused.submenu_ancestors().any(|a| a.id() == sub_self.id()) {
                            // keep open, focused child.
                            args.propagation().stop();
                            close_ancestors = None;
                        } else if sub_self.submenu_ancestors().any(|a| a.id() == sub_focused.id()) {
                            if Some(sub_focused.id()) == sub_self.submenu_parent().map(|s| s.id()) {
                                // keep open, focused parent.
                                args.propagation().stop();
                                close_ancestors = None;
                            } else {
                                close_ancestors = Some(Some(sub_focused.id()));
                            }
                        }
                    }

                    if let Some(sub_parent_focused) = close_ancestors {
                        // close any parent sub-menu that is not focused.
                        for a in sub_self.submenu_ancestors() {
                            if Some(a.id()) == sub_parent_focused {
                                break;
                            }

                            if let Some(v) = a.is_submenu_open() {
                                if v.get() {
                                    // request ancestor close the popup.
                                    POPUP_CLOSE_CMD.scoped(a.id()).notify();
                                }
                            } else if a.menu().is_none() {
                                // request context menu popup close
                                POPUP_CLOSE_CMD.scoped(a.id()).notify_param(PopupCloseMode::Force);
                            }
                        }
                    }
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update)
                && args.is_focus_leave(WIDGET.id())
                && let Some(f) = &args.new_focus
            {
                let info = WIDGET.info();
                let sub_self = if parent.is_some() {
                    info.submenu_parent()
                } else {
                    // is context menu
                    Some(info.clone())
                };
                if let Some(sub_menu) = sub_self
                    && let Some(f) = info.tree().get(f.widget_id())
                    && !f.submenu_self_and_ancestors().any(|s| s.id() == sub_menu.id())
                {
                    // Focus did not move to child sub-menu nor parent,
                    // close after delay.
                    //
                    // This covers the case of focus moving to a widget that is not
                    // a child sub-menu and is not the parent sub-menu,
                    // `sub_menu_node` covers the case of focus moving to the parent sub-menu and out.
                    let t = TIMERS.deadline(HOVER_OPEN_DELAY_VAR.get());
                    t.subscribe(UpdateOp::Update, info.id()).perm();
                    close_timer = Some(t);
                }
            }
        }
        UiNodeOp::Update { .. } => {
            if let Some(t) = &close_timer
                && t.get().has_elapsed()
            {
                close_timer = None;
                POPUP.force_close_id(WIDGET.id());
            }
        }
        _ => {}
    })
}
