//! Sub-menu widget and properties.

use zero_ui_core::focus::WidgetInfoFocusExt;

use crate::prelude::popup::{PopupState, POPUP};
use crate::prelude::{button, new_widget::*};

use crate::core::{
    focus::{FOCUS, FOCUS_CHANGED_EVENT},
    gesture::CLICK_EVENT,
    keyboard::{Key, KeyState, KEY_INPUT_EVENT},
    mouse::{ClickMode, MOUSE_HOVERED_EVENT},
    widget_info::WidgetInfo,
    widget_instance::ArcNodeList,
};

use super::ButtonStyle;

/// Submenu header and items.
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
            click_mode = ClickMode::press();
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
    let mut open = None::<ReadOnlyArcVar<PopupState>>;
    match_node(child, move |_, op| match op {
        UiNodeOp::Init => {
            WIDGET
                .sub_event(&CLICK_EVENT)
                .sub_event(&KEY_INPUT_EVENT)
                .sub_event(&FOCUS_CHANGED_EVENT)
                .sub_event(&MOUSE_HOVERED_EVENT);
        }
        UiNodeOp::Deinit => {
            let _ = IS_OPEN_VAR.set(false);
            if let Some(v) = open.take() {
                POPUP.force_close_var(v);
            }
        }
        UiNodeOp::Info { info } => {
            info.set_meta(
                &SUB_MENU_INFO_ID,
                SubMenuInfo {
                    parent: SUB_MENU_PARENT_CTX.get_clone(),
                },
            );
        }
        UiNodeOp::Event { update } => {
            let mut open_pop = false;

            if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                if args.is_mouse_enter() {
                    let info = WIDGET.info();

                    let is_root = info.parent_submenu().is_none();
                    let is_open = IS_OPEN_VAR.get();

                    // root sub-menus focus on hover only if the menu is focused or a sibling is open.
                    if is_root & !is_open {
                        if let (Some(menu), Some(focused)) = (info.menu(), FOCUS.focused().get()) {
                            let is_menu_focused = focused.contains(menu.id());

                            let mut focus_on_hover = is_menu_focused;
                            if !focus_on_hover {
                                if let Some(focused) = info.tree().get(focused.widget_id()) {
                                    if let Some(f_menu) = focused.menu() {
                                        // focused in menu-item, spawned from the same menu.
                                        focus_on_hover = f_menu.id() == menu.id();
                                    }
                                }
                            }

                            if focus_on_hover {
                                FOCUS.focus_widget(WIDGET.id(), false);
                            }
                        }
                    }
                }
                // TODO, auto-open.
                // - Context var that sets a timer.
                // - Is a delay by default in nested sub-menus.
                // - Is forever or zero
            } else if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                if let (Some(key), KeyState::Pressed) = (args.key, args.state) {
                    if !IS_OPEN_VAR.get() {
                        match key {
                            Key::Up | Key::Down => {
                                if let Some(info) = WIDGET.info().into_focusable(true, true) {
                                    open_pop = info.focusable_down().is_none() && info.focusable_up().is_none();
                                }
                            }
                            Key::Left | Key::Right => {
                                if let Some(info) = WIDGET.info().into_focusable(true, true) {
                                    open_pop = info.focusable_left().is_none() && info.focusable_right().is_none();
                                }
                            }
                            _ => {}
                        }

                        if open_pop {
                            args.propagation().stop();
                        }
                    }
                }
            } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                // TODO
                // - On focus, Open if sibling was open.
                // - On blur, close if descendant is not focused.
                if !IS_OPEN_VAR.get() && args.is_focus_enter(WIDGET.id()) {
                    let info = WIDGET.info();
                    if info.parent_submenu().is_none() {
                        if let Some(prev_menu) = args
                            .prev_focus
                            .as_ref()
                            .and_then(|p| info.tree().get(p.widget_id()))
                            .and_then(|w| w.menu())
                        {
                            if let Some(our_menu) = info.menu() {
                                open_pop = our_menu.id() == prev_menu.id();
                            }
                        }
                    }
                }
            } else if let Some(args) = CLICK_EVENT.on(update) {
                args.propagation().stop();

                open_pop = if let Some(v) = open.take() {
                    let closed = matches!(v.get(), PopupState::Closed);
                    if !closed {
                        POPUP.force_close_var(v);
                        FOCUS.focus_exit();
                    }
                    closed
                } else {
                    true
                };
                if !open_pop && open.is_none() {
                    let _ = IS_OPEN_VAR.set(false);
                }
            }

            if open_pop {
                let pop = super::popup::SubMenuPopup! {
                    parent_id = WIDGET.id();
                    children = children.take_on_init().boxed();
                };
                let state = POPUP.open(pop);
                let is_open = IS_OPEN_VAR.actual_var();
                let _ = is_open.set_from_map(&state, |s| !matches!(s, PopupState::Closed));
                state.bind_map(&is_open, |s| !matches!(s, PopupState::Closed)).perm();
                open = Some(state);
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
    pub static END_COLUMN_WIDTH_VAR: Length = 24;

    static IS_OPEN_VAR: bool = false;
}

/// If the sub-menu popup is open or opening.
#[property(CONTEXT, widget_impl(SubMenu))]
pub fn is_open(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    // reverse context var, is set by `sub_menu_node`.
    with_context_var(child, IS_OPEN_VAR, state)
}

/// Style applied to [`SubMenu!`] not inside any other sub-menus.
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

            when *#is_hovered || *#is_focused || *#is_open {
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

/// Extension methods for [`WidgetInfo`].
pub trait SubMenuWidgetInfoExt {
    /// If this widget is a [`SubMenu!`] instance.
    ///
    /// [`SubMenu!`]: struct@SubMenu
    fn is_submenu(&self) -> bool;

    /// Gets the sub-menu that spawned `self` if [`is_submenu`], otherwise returns the first ancestor
    /// that is sub-menu.
    ///
    /// Note that the returned widget may not be an actual parent in the info-tree as
    /// sub-menus use popups to present their sub-menus.
    ///
    /// [`is_submenu`]: SubMenuWidgetInfoExt::is_submenu
    fn parent_submenu(&self) -> Option<WidgetInfo>;

    /// Gets the parent submenu recursively, returns the parent that does not have a parent.
    fn root_submenu(&self) -> Option<WidgetInfo>;

    /// Gets the alt-scope parent of the `root_submenu`.
    fn menu(&self) -> Option<WidgetInfo>;
}
impl SubMenuWidgetInfoExt for WidgetInfo {
    fn is_submenu(&self) -> bool {
        self.meta().contains(&SUB_MENU_INFO_ID)
    }

    fn parent_submenu(&self) -> Option<WidgetInfo> {
        if let Some(p) = self.meta().get(&SUB_MENU_INFO_ID) {
            self.tree().get(p.parent?)
        } else {
            self.ancestors().find(|a| a.is_submenu())
        }
    }

    fn root_submenu(&self) -> Option<WidgetInfo> {
        find_root_submenu(self.clone())
    }

    fn menu(&self) -> Option<WidgetInfo> {
        let scope = self.root_submenu()?.into_focus_info(true, true).scope()?;

        if !scope.is_alt_scope() {
            return None;
        }

        Some(scope.info().clone())
    }
}

fn find_root_submenu(wgt: WidgetInfo) -> Option<WidgetInfo> {
    if let Some(parent) = wgt.parent_submenu() {
        find_root_submenu(parent)
    } else if wgt.is_submenu() {
        Some(wgt)
    } else {
        None
    }
}

pub(super) struct SubMenuInfo {
    pub parent: Option<WidgetId>,
}

context_local! {
    // only set during info
    pub(super) static SUB_MENU_PARENT_CTX: Option<WidgetId> = None;
}

pub(super) static SUB_MENU_INFO_ID: StaticStateId<SubMenuInfo> = StaticStateId::new_unique();
