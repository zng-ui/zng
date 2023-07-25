//! Sub-menu widget and properties.

use crate::prelude::popup::POPUP;
use crate::prelude::{button, new_widget::*};

use crate::core::{
    focus::{FOCUS, FOCUS_CHANGED_EVENT},
    gesture::CLICK_EVENT,
    keyboard::KEY_INPUT_EVENT,
    mouse::MOUSE_HOVERED_EVENT,
    widget_instance::ArcNodeList,
};

use super::ButtonStyle;

/// Submenu parent.
#[widget($crate::widgets::menu::sub::SubMenu)]
pub struct SubMenu(StyleMix<WidgetBase>);
impl SubMenu {
    widget_impl! {
        /// Sub-menu items.
        pub widget_base::children(children: impl UiNodeList);
    }

    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            style_fn = STYLE_VAR;
            focusable = true;
        }

        self.widget_builder().push_build_action(|wgt| {
            let header = wgt
                .capture_ui_node(property_id!(Self::header))
                .unwrap_or_else(|| FillUiNode.boxed());

            let children = wgt
                .capture_property(property_id!(Self::children))
                .map(|p| p.args.ui_node_list(0).clone())
                .unwrap_or_else(|| ArcNodeList::new(ui_vec![].boxed()));

            wgt.set_child(header);

            wgt.push_intrinsic(NestGroup::EVENT, "sub_menu_node", |c| sub_menu_node(c, children));
        });
    }
}

/// Sub-menu implementation.
pub fn sub_menu_node(child: impl UiNode, children: ArcNodeList<BoxedUiNodeList>) -> impl UiNode {
    let mut open = None;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_event(&CLICK_EVENT)
                .sub_event(&KEY_INPUT_EVENT)
                .sub_event(&FOCUS_CHANGED_EVENT)
                .sub_event(&MOUSE_HOVERED_EVENT);
        }
        UiNodeOp::Event { update } => {
            if let Some(_args) = MOUSE_HOVERED_EVENT.on(update) {
                // TODO, auto-open.
                // - Context var that sets a timer.
                // - Is a delay by default in nested sub-menus.
                // - Is forever or zero
            } else if let Some(_args) = KEY_INPUT_EVENT.on(update) {
                // TODO
                // - Special focus nav (when open left-right opens sibling menu)
            } else if let Some(_args) = FOCUS_CHANGED_EVENT.on(update) {
                // TODO
                // - On focus, Open if sibling was open.
                // - On blur, close if descendant is not focused.
            } else if let Some(args) = CLICK_EVENT.on(update) {
                args.propagation().stop();

                if let Some(open) = open.take() {
                    POPUP.force_close_var(open);
                    FOCUS.focus_exit();
                } else {
                    let pop_fn = POPUP_FN_VAR.get();
                    let pop = pop_fn(panel::PanelArgs {
                        children: children.take_on_init().boxed(),
                    });
                    open = Some(POPUP.open(pop));
                }
            }
        }
        _ => {}
    })
}

/// Sets the sub-menu style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the sub-menu style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Defines the sub-menu header child.
#[property(CHILD, capture, default(FillUiNode), widget_impl(SubMenu))]
pub fn header(child: impl UiNode) {}

/// Width of the icon/checkmark column.
///
/// This property sets [`START_COLUMN_WIDTH_VAR`].
#[property(CONTEXT, default(START_COLUMN_WIDTH_VAR), widget_impl(SubMenu))]
pub fn start_column_width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, START_COLUMN_WIDTH_VAR, width)
}

/// Width of the sub-menu expand symbol column.
///
/// This property sets [`END_COLUMN_WIDTH_VAR`].
#[property(CONTEXT, default(END_COLUMN_WIDTH_VAR), widget_impl(SubMenu))]
pub fn end_column_width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, END_COLUMN_WIDTH_VAR, width)
}

/// Sets the content to the [`Align::START`] side of the button menu item.
///
/// The `cell` is an non-interactive background that fills the [`START_COLUMN_WIDTH_VAR`] and button height.
///
/// This is usually an icon, or a checkmark.
#[property(FILL)]
pub fn start_column(child: impl UiNode, cell: impl UiNode) -> impl UiNode {
    let cell = width(cell, START_COLUMN_WIDTH_VAR);
    let cell = align(cell, Align::FILL_START);
    background(child, cell)
}

/// Sets the icon of a button inside the menu.
#[property(FILL)]
pub fn end_column(child: impl UiNode, cell: impl UiNode) -> impl UiNode {
    let cell = width(cell, END_COLUMN_WIDTH_VAR);
    let cell = align(cell, Align::FILL_END);
    background(child, cell)
}

/// If the start and end column width is applied as padding.
///
/// This property is enabled in menu-item styles to offset the content by [`start_column_width`] and [`end_column_width`].
///
/// [`start_column_width`]: fn@start_column_width
/// [`end_column_width`]: fn@end_column_width
#[property(CHILD_LAYOUT, default(false))]
pub fn column_width_padding(child: impl UiNode, enabled: impl IntoVar<bool>) -> impl UiNode {
    let spacing = merge_var!(
        START_COLUMN_WIDTH_VAR,
        END_COLUMN_WIDTH_VAR,
        DIRECTION_VAR,
        enabled.into_var(),
        |s, e, d, enabled| {
            if *enabled {
                let s = s.clone();
                let e = e.clone();
                if d.is_ltr() {
                    SideOffsets::new(0, e, 0, s)
                } else {
                    SideOffsets::new(0, s, 0, e)
                }
            } else {
                SideOffsets::zero()
            }
        }
    );
    padding(child, spacing)
}

/// Widget function that generates the sub-menu popup and layout panel.
///
/// This property can be set in any widget to affect all sub-menu popup children descendants.
///
/// This property sets [`PANEL_FN_VAR`].
#[property(CONTEXT, default(POPUP_FN_VAR), widget_impl(SubMenu))]
pub fn popup_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<panel::PanelArgs>>) -> impl UiNode {
    with_context_var(child, POPUP_FN_VAR, panel)
}

context_var! {
    /// Sub-menu style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// Width of the icon/checkmark column.
    pub static START_COLUMN_WIDTH_VAR: Length = 32;

    /// Width of the sub-menu expand symbol column.
    pub static END_COLUMN_WIDTH_VAR: Length = 32;

    /// Defines the popup and layout widget for used to present the sub-menu items.
    ///
    /// Is a [`Popup!`] wrapping a [`Scroll!`] wrapping a [`Stack!`] panel by default.
    ///
    /// [`Popup!`]: struct@crate::widgets::popup::Popup
    /// [`Scroll!`]: struct@crate::widgets::Scroll
    /// [`Stack!`]: struct@crate::widgets::layouts::Stack
    pub static POPUP_FN_VAR: WidgetFn<panel::PanelArgs> = wgt_fn!(|a: panel::PanelArgs| {
        crate::widgets::popup::Popup! {
            replace_style = SubMenuStyle!();

            child = crate::widgets::Scroll! {
                child = crate::widgets::layouts::Stack! {
                    children = a.children;
                    direction = crate::widgets::layouts::stack::StackDirection::top_to_bottom();
                };
                mode = crate::widgets::scroll::ScrollMode::VERTICAL;
            };
        }
    });
}

/// Style applied to all [`SubMenu!`] not inside any other sub-menu.
///
/// [`SubMenu!`]: struct@SubMenu
/// [`Menu!`]: struct@Menu
#[widget($crate::widgets::menu::sub::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            padding = (4, 10);
            opacity = 90.pct();
            foreground_highlight = unset!;

            when *#is_hovered || *#is_focused {
                background_color = button::color_scheme_hovered(button::BASE_COLORS_VAR);
                opacity = 100.pct();
            }

            when *#is_disabled {
                saturate = false;
                opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }
        }
    }
}

/// Style applied to all [`SubMenu!`] widgets inside other sub-menus.
#[widget($crate::widgets::menu::sub::SubMenuStyle)]
pub struct SubMenuStyle(ButtonStyle);
impl SubMenuStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            end_column = crate::widgets::Text! {
                size = 1.2.em();
                font_family = FontNames::system_ui(&lang!(und));
                align = Align::CENTER;

                txt = "⏵";
                when *#is_rtl {
                    txt = "⏴";
                }
            }
        }
    }
}
