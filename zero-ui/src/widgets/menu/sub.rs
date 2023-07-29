//! Sub-menu widget and properties.

use std::time::Duration;

use crate::{
    core::{
        focus::{WidgetInfoFocusExt, FOCUS, FOCUS_CHANGED_EVENT},
        gesture::CLICK_EVENT,
        keyboard::{Key, KeyState, KEY_INPUT_EVENT},
        mouse::{ClickMode, MOUSE_HOVERED_EVENT},
        timer::TIMERS,
        widget_info::WidgetInfo,
        widget_instance::ArcNodeList,
    },
    prelude::{
        button,
        layers::AnchorSize,
        new_widget::*,
        popup::{PopupState, POPUP, POPUP_CLOSE_CMD},
        AnchorMode, AnchorOffset,
    },
};

use super::ButtonStyle;

/// Submenu header and items.
#[widget($crate::widgets::menu::sub::SubMenu {
    ($header_txt:expr, $children:expr $(,)?) => {
        header = $crate::widgets::Text!($header_txt);
        children = $children;
    }
})]
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
            focus_click_behavior = FocusClickBehavior::Ignore; // we handle clicks.
            capture_mouse = true; // part of press-and-drag to click (see SubMenuPopup)
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
    let is_open = var(false);
    let mut open_timer = None;
    let mut close_timer = None;
    let child = with_context_var(child, IS_OPEN_VAR, is_open.clone());
    let mut close_cmd = CommandHandle::dummy();

    match_node(child, move |_, op| {
        let mut open_pop = false;

        match op {
            UiNodeOp::Init => {
                WIDGET
                    .sub_event(&CLICK_EVENT)
                    .sub_event(&KEY_INPUT_EVENT)
                    .sub_event(&FOCUS_CHANGED_EVENT)
                    .sub_event(&MOUSE_HOVERED_EVENT);

                close_cmd = POPUP_CLOSE_CMD.scoped(WIDGET.id()).subscribe(false);
            }
            UiNodeOp::Deinit => {
                if let Some(v) = open.take() {
                    POPUP.force_close_var(v);
                    is_open.set(false);
                }
                close_cmd = CommandHandle::dummy();
                open_timer = None;
                close_timer = None;
            }
            UiNodeOp::Info { info } => {
                info.set_meta(
                    &SUB_MENU_INFO_ID,
                    SubMenuInfo {
                        parent: SUB_MENU_PARENT_CTX.get_clone(),
                        is_open: is_open.clone(),
                    },
                );
            }
            UiNodeOp::Event { update } => {
                if let Some(args) = MOUSE_HOVERED_EVENT.on(update) {
                    if args.is_mouse_enter() {
                        let info = WIDGET.info();

                        let is_root = info.submenu_parent().is_none();
                        let is_open = is_open.get();

                        if is_root {
                            // menus focus on hover (implemented in sub_menu_popup_node)
                            // root sub-menus focus on hover only if the menu is focused or a sibling is open (implemented here)

                            if !is_open {
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
                                        // focus, the popup will open on FOCUS_CHANGED_EVENT too.
                                        FOCUS.focus_widget(WIDGET.id(), false);
                                    }
                                }
                            }
                        } else if !is_open && open_timer.is_none() {
                            // non-root sub-menus open after a hover delay.
                            let t = TIMERS.deadline(HOVER_OPEN_DELAY_VAR.get());
                            t.subscribe(UpdateOp::Update, WIDGET.id()).perm();
                            open_timer = Some(t);
                        }
                    } else if args.is_mouse_leave() {
                        open_timer = None;
                    }
                } else if let Some(args) = KEY_INPUT_EVENT.on_unhandled(update) {
                    if let (Some(key), KeyState::Pressed) = (args.key, args.state) {
                        if !is_open.get() {
                            if let Some(info) = WIDGET.info().into_focusable(true, true) {
                                if info.info().submenu_parent().is_none() {
                                    // root, open for arrow keys that do not cause focus move
                                    if matches!(key, Key::Up | Key::Down) {
                                        open_pop = info.focusable_down().is_none() && info.focusable_up().is_none();
                                    } else if matches!(key, Key::Left | Key::Right) {
                                        open_pop = info.focusable_left().is_none() && info.focusable_right().is_none();
                                    }
                                } else {
                                    // sub, open in direction.
                                    match DIRECTION_VAR.get() {
                                        LayoutDirection::LTR => open_pop = matches!(key, Key::Right),
                                        LayoutDirection::RTL => open_pop = matches!(key, Key::Left),
                                    }
                                }
                            }

                            if open_pop {
                                args.propagation().stop();
                            }
                        }
                    }
                } else if let Some(args) = FOCUS_CHANGED_EVENT.on(update) {
                    if args.is_focus_enter(WIDGET.id()) {
                        close_timer = None;
                        if !is_open.get() {
                            // focused when not open
                            let info = WIDGET.info();
                            if info.submenu_parent().is_none() {
                                // is root sub-menu
                                if let Some(prev_root) = args
                                    .prev_focus
                                    .as_ref()
                                    .and_then(|p| info.tree().get(p.widget_id()))
                                    .and_then(|w| w.submenu_root())
                                {
                                    // prev focus was open
                                    if prev_root.is_submenu_open().map(|v| v.get()).unwrap_or(false) {
                                        if let (Some(prev_menu), Some(our_menu)) = (prev_root.menu(), info.menu()) {
                                            // same menu and sibling was open, open
                                            open_pop = our_menu.id() == prev_menu.id();
                                        }
                                    }
                                }
                            }
                        }
                    } else if args.is_focus_leave(WIDGET.id()) && is_open.get() {
                        if let Some(f) = &args.new_focus {
                            if let Some(f) = WINDOW.info().get(f.widget_id()) {
                                let id = WIDGET.id();
                                if !f.submenu_ancestors().any(|s| s.id() == id) {
                                    // Focus did not move to child sub-menu,
                                    // close after delay.
                                    //
                                    // This covers the case of focus moving back to the sub-menu and then away,
                                    // `sub_menu_popup_node` covers the case of focus moving to a different sub-menu directly.
                                    let t = TIMERS.deadline(HOVER_OPEN_DELAY_VAR.get());
                                    t.subscribe(UpdateOp::Update, id).perm();
                                    close_timer = Some(t);
                                }
                            }
                        }
                    }
                } else if let Some(args) = CLICK_EVENT.on(update) {
                    args.propagation().stop();

                    // open if is closed
                    open_pop = if let Some(s) = open.take() {
                        let closed = matches!(s.get(), PopupState::Closed);
                        if !closed {
                            if WIDGET.info().submenu_parent().is_none() {
                                // root sub-menu, close and return focus
                                POPUP.force_close_var(s);
                                FOCUS.focus_exit();
                                is_open.set(false);
                                close_cmd.set_enabled(false);
                            } else {
                                // nested sub-menu.
                                open = Some(s);
                            }
                        }
                        closed
                    } else {
                        true
                    };
                    if !open_pop && open.is_none() {
                        is_open.set(false);
                    }
                } else if let Some(_args) = POPUP_CLOSE_CMD.scoped(WIDGET.id()).on(update) {
                    if let Some(s) = open.take() {
                        if !matches!(s.get(), PopupState::Closed) {
                            POPUP.force_close_var(s);
                            is_open.set(false);
                            close_cmd.set_enabled(false);
                        }
                    }
                }
            }
            UiNodeOp::Update { .. } => {
                if let Some(s) = &open {
                    if matches!(s.get(), PopupState::Closed) {
                        is_open.set(false);
                        close_cmd.set_enabled(false);
                        close_timer = None;
                        open = None;
                    } else if let Some(t) = &close_timer {
                        if t.get().has_elapsed() {
                            if let Some(s) = open.take() {
                                if !matches!(s.get(), PopupState::Closed) {
                                    POPUP.force_close_var(s);
                                    is_open.set(false);
                                    close_cmd.set_enabled(false);
                                }
                            }
                            close_timer = None;
                        }
                    }
                } else if let Some(t) = &open_timer {
                    if t.get().has_elapsed() {
                        open_pop = true;
                    }
                }
            }
            _ => {}
        }
        if open_pop {
            let pop = super::popup::SubMenuPopup! {
                parent_id = WIDGET.id();
                children = children.take_on_init().boxed();
            };
            let state = POPUP.open(pop);
            state.subscribe(UpdateOp::Update, WIDGET.id()).perm();
            if !matches!(state.get(), PopupState::Closed) {
                is_open.set(true);
                close_cmd.set_enabled(true);
            }
            open = Some(state);
            open_timer = None;
        }
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
///
/// See also [`start_column_fn`] for use in styles.
///
/// [`start_column_fn`]: fn@start_column_fn
#[property(FILL)]
pub fn start_column(child: impl UiNode, cell: impl UiNode) -> impl UiNode {
    let cell = width(cell, START_COLUMN_WIDTH_VAR);
    let cell = align(cell, Align::FILL_START);
    background(child, cell)
}

/// Sets the content to the [`Align::END`] side of the button menu item.
///
/// The `cell` is an non-interactive background that fills the [`END_COLUMN_WIDTH_VAR`] and button height.
///
/// This is usually a little arrow for sub-menus.
///
/// See also [`end_column_fn`] for use in styles.
///
/// [`end_column_fn`]: fn@end_column_fn
#[property(FILL)]
pub fn end_column(child: impl UiNode, cell: impl UiNode) -> impl UiNode {
    let cell = width(cell, END_COLUMN_WIDTH_VAR);
    let cell = align(cell, Align::FILL_END);
    background(child, cell)
}

/// Sets the content to the [`Align::START`] side of the button menu item generated using a [`WidgetFn<()>`].
///
/// This property presents the same visual as [`start_column`], but when used in styles `cell_fn` is called
/// multiple times to generate duplicates of the start cell.
///
/// [`start_column`]: fn@start_column
#[property(FILL)]
pub fn start_column_fn(child: impl UiNode, cell_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    start_column(child, presenter((), cell_fn))
}

/// Sets the content to the [`Align::END`] side of the button menu item generated using a [`WidgetFn<()>`].
///
/// This property presents the same visual as [`end_column`], but when used in styles `cell_fn` is called
/// multiple times to generate duplicates of the start cell.
///
/// [`end_column`]: fn@end_column
#[property(FILL)]
pub fn end_column_fn(child: impl UiNode, cell_fn: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    end_column(child, presenter((), cell_fn))
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

    /// Delay a sub-menu must be hovered to open the popup.
    ///
    /// Is `300.ms()` by default.
    pub static HOVER_OPEN_DELAY_VAR: Duration = 300.ms();

    static IS_OPEN_VAR: bool = false;
}

/// If the sub-menu popup is open or opening.
#[property(EVENT, widget_impl(SubMenu))]
pub fn is_open(child: impl UiNode, state: impl IntoVar<bool>) -> impl UiNode {
    bind_is_state(child, IS_OPEN_VAR, state)
}

/// Delay a sub-menu must be hovered to open the popup.
///
/// Is `300.ms()` by default.
///
/// This property sets the [`HOVER_OPEN_DELAY_VAR`].
#[property(CONTEXT, default(HOVER_OPEN_DELAY_VAR), widget_impl(SubMenu))]
pub fn hover_open_delay(child: impl UiNode, delay: impl IntoVar<Duration>) -> impl UiNode {
    with_context_var(child, HOVER_OPEN_DELAY_VAR, delay)
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

            crate::widgets::popup::anchor_mode = DIRECTION_VAR.map(|d| match d {
                LayoutDirection::LTR => AnchorMode::popup(AnchorOffset { place: Point::bottom_left(), origin: Point::top_left() }),
                LayoutDirection::RTL => AnchorMode::popup(AnchorOffset { place: Point::bottom_right(), origin: Point::top_right() }),
            });

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

            crate::widgets::popup::anchor_mode = DIRECTION_VAR.map(|d| match d {
                LayoutDirection::LTR => AnchorMode::popup(AnchorOffset { place: Point::top_right(), origin: Point::top_left() }),
                LayoutDirection::RTL => AnchorMode::popup(AnchorOffset { place: Point::top_left(), origin: Point::top_right() }),
            }.with_min_size(AnchorSize::Unbounded));

            when *#is_open {
                background_color = button::color_scheme_hovered(button::BASE_COLORS_VAR);
                opacity = 100.pct();
            }


            end_column_fn = wgt_fn!(|_|crate::widgets::Text! {
                size = 1.2.em();
                font_family = FontNames::system_ui(&lang!(und));
                align = Align::CENTER;

                txt = "⏵";
                when *#is_rtl {
                    txt = "⏴";
                }
            })
        }
    }
}

/// Extension methods for [`WidgetInfo`].
pub trait SubMenuWidgetInfoExt {
    /// If this widget is a [`SubMenu!`] instance.
    ///
    /// [`SubMenu!`]: struct@SubMenu
    fn is_submenu(&self) -> bool;

    /// Gets a variable that tracks if the sub-menu is open.
    fn is_submenu_open(&self) -> Option<ReadOnlyArcVar<bool>>;

    /// Gets the sub-menu that spawned `self` if [`is_submenu`], otherwise returns the first ancestor
    /// that is sub-menu.
    ///
    /// Note that the returned widget may not be an actual parent in the info-tree as
    /// sub-menus use popups to present their sub-menus.
    ///
    /// [`is_submenu`]: SubMenuWidgetInfoExt::is_submenu
    fn submenu_parent(&self) -> Option<WidgetInfo>;

    /// Gets an iterator over sub-menu parents until root.
    fn submenu_ancestors(&self) -> SubMenuAncestors;
    /// Gets an iterator over the widget, if it is a sub-menu, and sub-menu parents until root.
    fn submenu_self_and_ancestors(&self) -> SubMenuAncestors;

    /// Gets the last submenu ancestor.
    fn submenu_root(&self) -> Option<WidgetInfo>;

    /// Gets the alt-scope parent of the `root_submenu`.
    fn menu(&self) -> Option<WidgetInfo>;
}
impl SubMenuWidgetInfoExt for WidgetInfo {
    fn is_submenu(&self) -> bool {
        self.meta().contains(&SUB_MENU_INFO_ID)
    }

    fn is_submenu_open(&self) -> Option<ReadOnlyArcVar<bool>> {
        self.meta().get(&SUB_MENU_INFO_ID).map(|s| s.is_open.read_only())
    }

    fn submenu_parent(&self) -> Option<WidgetInfo> {
        if let Some(p) = self.meta().get(&SUB_MENU_INFO_ID) {
            self.tree().get(p.parent?)
        } else if let Some(p) = self.ancestors().find(|a| a.is_submenu()) {
            Some(p)
        } else if let Some(pop) = self.meta().get(&SUB_MENU_POPUP_ID) {
            self.tree().get(pop.parent?)
        } else {
            for anc in self.ancestors() {
                if let Some(pop) = anc.meta().get(&SUB_MENU_POPUP_ID) {
                    return self.tree().get(pop.parent?);
                }
            }
            None
        }
    }

    fn submenu_ancestors(&self) -> SubMenuAncestors {
        SubMenuAncestors {
            node: self.submenu_parent(),
        }
    }

    fn submenu_self_and_ancestors(&self) -> SubMenuAncestors {
        if self.is_submenu() {
            SubMenuAncestors { node: Some(self.clone()) }
        } else {
            self.submenu_ancestors()
        }
    }

    fn submenu_root(&self) -> Option<WidgetInfo> {
        self.submenu_ancestors().last()
    }

    fn menu(&self) -> Option<WidgetInfo> {
        let root = self
            .submenu_root()
            .or_else(|| if self.is_submenu() { Some(self.clone()) } else { None })?;

        let scope = root.into_focus_info(true, true).scope()?;

        if !scope.is_alt_scope() {
            return None;
        }

        Some(scope.info().clone())
    }
}

/// Iterator over sub-menu parents.
///
/// See [`submenu_ancestors`] for more details.
///
/// [`submenu_ancestors`]: SubMenuWidgetInfoExt::submenu_ancestors
pub struct SubMenuAncestors {
    node: Option<WidgetInfo>,
}
impl Iterator for SubMenuAncestors {
    type Item = WidgetInfo;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(n) = self.node.take() {
            self.node = n.submenu_parent();
            Some(n)
        } else {
            None
        }
    }
}

pub(super) struct SubMenuInfo {
    pub parent: Option<WidgetId>,
    pub is_open: ArcVar<bool>,
}

pub(super) struct SubMenuPopupInfo {
    pub parent: Option<WidgetId>,
}

context_local! {
    // only set during info
    pub(super) static SUB_MENU_PARENT_CTX: Option<WidgetId> = None;
}

pub(super) static SUB_MENU_INFO_ID: StaticStateId<SubMenuInfo> = StaticStateId::new_unique();
pub(super) static SUB_MENU_POPUP_ID: StaticStateId<SubMenuPopupInfo> = StaticStateId::new_unique();