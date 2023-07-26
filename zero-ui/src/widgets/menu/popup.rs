//! Sub-menu popup widget and properties.

use zero_ui_core::{
    gesture::{CommandShortcutExt, Shortcuts},
    keyboard::{Key, KeyState, KEY_INPUT_EVENT},
    widget_instance::ArcNodeList,
};

use crate::prelude::{button, new_widget::*, scroll};

/// Sub-menu popup.
#[widget($crate::widgets::menu::popup::SubMenuPopup)]
pub struct SubMenuPopup(crate::widgets::popup::Popup);
impl SubMenuPopup {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;
        }

        self.widget_builder().push_build_action(|wgt| {
            if let Some(id) = wgt.capture_value::<WidgetId>(property_id!(Self::parent_id)) {
                let children = wgt
                    .capture_property(property_id!(Self::children))
                    .map(|p| p.args.ui_node_list(0).clone())
                    .unwrap_or_else(|| ArcNodeList::new(ui_vec![].boxed()));

                wgt.set_child(sub_menu_popup_node(children, id))
            } else {
                tracing::error!("`parent_id` is required");
            }
        });
    }

    widget_impl! {
        /// Sub-menu items.
        pub crate::core::widget_base::children(children: impl UiNodeList);
    }
}

/// Parent sub-menu ID.
#[property(CONTEXT, capture, widget_impl(SubMenuPopup))]
pub fn parent_id(submenu_id: impl IntoValue<WidgetId>) {}

context_var! {
    /// Defines the layout widget for [`SubMenuPopup!`].
    ///
    /// Is [`default_panel_fn`] by default.
    ///
    /// [`SubMenuPopup!`]: struct@SubMenuPopup
    pub static PANEL_FN_VAR: WidgetFn<panel::PanelArgs> = WidgetFn::new(default_panel_fn);

    /// Sub-menu popup style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
}

/// Widget function that generates the menu layout.
///
/// This property sets [`PANEL_FN_VAR`].
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(SubMenuPopup))]
pub fn panel_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<panel::PanelArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Sets the sub-menu popup style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the sub-menu popup style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Sub-menu popup default style.
#[widget($crate::widgets::menu::popup::DefaultStyle)]
pub struct DefaultStyle(crate::widgets::popup::DefaultStyle);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            super::sub::replace_style = super::sub::SubMenuStyle!();

            border = {
                widths: 1,
                sides: button::color_scheme_hovered(button::BASE_COLORS_VAR).map_into(),
            };
        }
    }
}

/// Default sub-menu popup panel view.
///
/// See [`PANEL_FN_VAR`] for more details.
pub fn default_panel_fn(args: panel::PanelArgs) -> impl UiNode {
    // remove arrow key shortcuts, they are used to nav. focus.
    let scroll_id = WidgetId::new_unique();
    let _ = scroll::commands::SCROLL_UP_CMD.scoped(scroll_id).shortcut().set(Shortcuts::new());
    let _ = scroll::commands::SCROLL_DOWN_CMD.scoped(scroll_id).shortcut().set(Shortcuts::new());

    crate::widgets::Scroll! {
        id = scroll_id;
        focusable = false;
        child = crate::widgets::layouts::Stack! {
            children = args.children;
            direction = crate::widgets::layouts::stack::StackDirection::top_to_bottom();
        };
        mode = scroll::ScrollMode::VERTICAL;
    }
}

/// Sub-menu popup implementation.
pub fn sub_menu_popup_node(children: ArcNodeList<BoxedUiNodeList>, parent: WidgetId) -> impl UiNode {
    let child = crate::widgets::layouts::panel::node(children, PANEL_FN_VAR);
    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_event(&KEY_INPUT_EVENT);
        }
        UiNodeOp::Info { info } => {
            super::sub::SUB_MENU_PARENT_CTX.with_context_value(Some(parent), || c.info(info));
            info.set_meta(&super::sub::SUB_MENU_INFO_ID, super::sub::SubMenuInfo { parent: Some(parent) });
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            let args = c
                .with_context(WidgetUpdateMode::Bubble, || KEY_INPUT_EVENT.on_unhandled(update))
                .flatten();

            if let Some(args) = args {
                if let (Some(key), KeyState::Pressed) = (args.key, args.state) {
                    if let Key::Left | Key::Right = key {
                        // TODO, return to parent or open root parent next menu.

                        // if let Some(info) = WIDGET.info().into_focusable(true, true) {
                        //     if info.focusable_left().is_none() && info.focusable_right().is_none() {
                        //         if let Some(parent) = info.info().parent_submenu() {
                        //             if let Some(orientation) = parent.orientation_from(info.info().center()) {
                        //                 match key {
                        //                     _ => {}
                        //                 }
                        //                 match orientation {
                        //                     Orientation2D::Left => todo!(),
                        //                     Orientation2D::Right => todo!(),
                        //                     _ => {}
                        //                 }
                        //             }
                        //         }
                        //     }
                        // }
                    }
                }
            }
        }
        _ => {}
    })
}
