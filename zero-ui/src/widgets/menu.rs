//! Menu widgets and properties.
//!

use crate::{
    core::{focus::FOCUS, gesture::CLICK_EVENT, mouse::ClickMode},
    prelude::{button, events::mouse::on_pre_mouse_enter, new_widget::*, rule_line::hr, toggle, AnchorMode},
};

use super::popup::{PopupState, POPUP};

pub mod popup;
pub mod sub;

/// Menu root panel.
#[widget($crate::widgets::menu::Menu {
    ($children:expr) => {
        children = $children;
    }
})]
pub struct Menu(StyleMix<panel::Panel>);
impl Menu {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            alt_focus_scope = true;
            panel::panel_fn = PANEL_FN_VAR;
            style_fn = STYLE_VAR;
        }
    }
}

context_var! {
    /// Defines the layout widget for [`Menu!`].
    ///
    /// Is a [`Wrap!`] panel by default.
    ///
    /// [`Panel!`]: struct@Panel
    /// [`Wrap!`]: struct@crate::widgets::layouts::Wrap
    pub static PANEL_FN_VAR: WidgetFn<panel::PanelArgs> = wgt_fn!(|a: panel::PanelArgs| {
        crate::widgets::layouts::Wrap! {
            children = a.children;
        }
    });

    /// Menu style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());

    /// Minimum space between a menu item child and the [`shortcut_txt`] child.
    ///
    /// The spacing is applied only if the shortcut child is set to a non-zero sized widget and
    /// there is no other wider menu item.
    ///
    /// Is `20` by default.
    ///
    /// [`shortcut_txt`]: fn@shortcut_txt
    pub static SHORTCUT_SPACING_VAR: Length = 20;
}

/// Widget function that generates the menu layout.
///
/// This property can be set in any widget to affect all [`Menu!`] descendants.
///
/// This property sets [`PANEL_FN_VAR`].
///
/// [`Menu!`]: struct@Menu
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(Menu))]
pub fn panel_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<panel::PanelArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Sets the menu style in a context, the parent style is fully replaced.
#[property(CONTEXT, default(STYLE_VAR))]
pub fn replace_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    with_context_var(child, STYLE_VAR, style)
}

/// Extends the menu style in a context, the parent style is used, properties of the same name set in
/// `style` override the parent style.
#[property(CONTEXT, default(StyleFn::nil()))]
pub fn extend_style(child: impl UiNode, style: impl IntoVar<StyleFn>) -> impl UiNode {
    style::with_style_extension(child, STYLE_VAR, style)
}

/// Default [`Menu!`] style.
///
/// Gives the button a *menu-item* look.
#[widget($crate::widgets::menu::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            button::replace_style = style_fn!(|_| ButtonStyle!());
            toggle::replace_style = style_fn!(|_| ToggleStyle!());
            hr::color = button::color_scheme_hovered(button::BASE_COLORS_VAR);
            crate::widgets::icon::ico_size = 18;
        }
    }
}

/// Style applied to all [`Button!`] widgets inside [`Menu!`].
///
/// Gives the button a *menu-item* look.
#[widget($crate::widgets::menu::ButtonStyle)]
pub struct ButtonStyle(Style);
impl ButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            sub::column_width_padding = true;
            padding = (4, 0);
            child_align = Align::START;

            background_color = color_scheme_pair(button::BASE_COLORS_VAR);
            opacity = 90.pct();
            foreground_highlight = unset!;

            click_mode = ClickMode::release();// part of press-and-drag to click (see SubMenuPopup)

            on_pre_mouse_enter = hn!(|_| {
                FOCUS.focus_widget(WIDGET.id(), false);
            });

            when *#is_focused {
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

/// Style applied to all [`Button!`] widgets inside [`Menu!`].
///
/// Gives the toggle a *menu-item* look, the checkmark is placed in the icon position.
#[widget($crate::widgets::menu::ToggleStyle)]
pub struct ToggleStyle(ButtonStyle);
impl ToggleStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            click_mode = ClickMode::release();

            sub::start_column_fn = wgt_fn!(|_ |crate::widgets::Text! {
                size = 1.2.em();
                font_family = FontNames::system_ui(&lang!(und));
                align = Align::CENTER;

                txt = "✓";
                when #{toggle::IS_CHECKED_VAR}.is_none() {
                    txt = "━";
                }

                font_color = text::FONT_COLOR_VAR.map(|c| c.transparent());
                when #{toggle::IS_CHECKED_VAR}.unwrap_or(true) {
                    font_color = text::FONT_COLOR_VAR;
                }
            })
        }
    }
}

/// Menu item icon.
///
/// Set on a [`Button!`] inside a sub-menu to define the menu [`Icon!`] for that button.
///
/// This property is an alias for [`sub::start_column`].
///
/// [`Button!`]: struct@crate::widgets::Button
/// [`Icon!`]: struct@crate::widgets::Icon
/// [`sub::start_column`]: fn@sub::start_column
#[property(FILL)]
pub fn icon(child: impl UiNode, icon: impl UiNode) -> impl UiNode {
    sub::start_column(child, icon)
}

/// Menu item icon from widget function.
///
/// Set on a [`Button!`] inside a sub-menu to define the menu [`Icon!`] for that button.
///
/// This property is an alias for [`sub::start_column_fn`].
///
/// [`Button!`]: struct@crate::widgets::Button
/// [`Icon!`]: struct@crate::widgets::Icon
/// [`sub::start_column_fn`]: fn@sub::start_column_fn
#[property(FILL)]
pub fn icon_fn(child: impl UiNode, icon: impl IntoVar<WidgetFn<()>>) -> impl UiNode {
    sub::start_column_fn(child, icon)
}

/// Menu item shortcut text.
///
/// Set on a [`Button!`] inside a sub-menu to define the shortcut text.
///
/// Note that this does not define the click shortcut, just the display of it.
///
/// [`Button!`]: struct@crate::widgets::Button
#[property(CHILD_CONTEXT)]
pub fn shortcut_txt(child: impl UiNode, shortcut: impl UiNode) -> impl UiNode {
    let shortcut = margin(shortcut, sub::END_COLUMN_WIDTH_VAR.map(|w| SideOffsets::new(0, w.clone(), 0, 0)));
    child_insert_end(child, shortcut, SHORTCUT_SPACING_VAR)
}

/// Minimum space between a menu item child and the [`shortcut_txt`] child.
///
/// The spacing is applied only if the shortcut child is set to a non-zero sized widget and
/// there is no other wider menu item.
///
/// This property sets the [`SHORTCUT_SPACING_VAR`].
///
/// [`shortcut_txt`]: fn@shortcut_txt
#[property(CONTEXT, default(SHORTCUT_SPACING_VAR))]
pub fn shortcut_spacing(child: impl UiNode, spacing: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, SHORTCUT_SPACING_VAR, spacing)
}

/// Command button.
///
/// This a menu button that has a `cmd` property, if the property is set the button `child`, `icon_fn` and `shortcut_txt`
/// are set with values from the command metadata, the `on_click` handle is set to call the command and the `enable` and
/// `visibility` are set from the command handle status.
#[widget($crate::widgets::menu::CmdButton {
    ($cmd:expr) => {
        cmd = $cmd;
    }
})]
pub struct CmdButton(crate::widgets::Button);
impl CmdButton {
    /// Build the button from the `cmd` value.
    pub fn widget_build(&mut self) -> impl UiNode {
        use crate::prelude::*;
        use crate::widgets::icon::CommandIconExt;

        if let Some(cmd) = self.widget_builder().capture_value::<Command>(property_id!(Self::cmd)) {
            widget_set! {
                self;

                enabled = cmd.is_enabled();
                visibility = cmd.has_handlers().map_into();

                child = Text!(cmd.name());

                shortcut_txt = Text!(cmd.shortcut_txt());
                icon_fn = cmd.icon();

                on_click = hn!(|args: &ClickArgs| {
                    if cmd.is_enabled_value() {
                        args.propagation().stop();
                        cmd.notify();
                    }
                });
            }
        }

        let base: &mut WidgetBase = self;
        base.widget_build()
    }
}

/// The button command.
#[property(CONTEXT, capture, widget_impl(CmdButton))]
pub fn cmd(cmd: impl IntoValue<Command>) {}

/// Defines the context menu shown when the widget is enabled and receives a context click.
///
/// The `menu` can be any widget, the [`ContextMenu!`] is recommended. The menu widget is open
/// using [`POPUP`] and is expected to close itself when the context action is finished or it
/// loses focus.
///
/// [`ContextMenu!`]: struct@ContextMenu
#[property(EVENT)]
pub fn context_menu(child: impl UiNode, menu: impl UiNode) -> impl UiNode {
    context_menu_fn(child, WidgetFn::singleton(menu))
}

/// Defines the context menu function shown when the widget is enabled and receives a context click.
///
/// The `menu` can return any widget, the [`ContextMenu!`] is recommended.
///
/// [`ContextMenu!`]: struct@ContextMenu
#[property(EVENT, default(WidgetFn::nil()))]
pub fn context_menu_fn(child: impl UiNode, menu: impl IntoVar<WidgetFn<ContextMenuArgs>>) -> impl UiNode {
    context_menu_node(child, menu, false)
}

/// Defines the context menu shown when the widget is disabled and receives a context click.
///
/// The `menu` can be any widget, the [`ContextMenu!`] is recommended.
///
/// [`ContextMenu!`]: struct@ContextMenu
#[property(EVENT)]
pub fn disabled_context_menu(child: impl UiNode, menu: impl UiNode) -> impl UiNode {
    disabled_context_menu_fn(child, WidgetFn::singleton(menu))
}

/// Defines the context menu function shown when the widget is disabled and receives a context click.
///
/// The `menu` can return any widget, the [`ContextMenu!`] is recommended.
///
/// [`ContextMenu!`]: struct@ContextMenu
#[property(EVENT, default(WidgetFn::nil()))]
pub fn disabled_context_menu_fn(child: impl UiNode, menu: impl IntoVar<WidgetFn<ContextMenuArgs>>) -> impl UiNode {
    context_menu_node(child, menu, true)
}

fn context_menu_node(child: impl UiNode, menu: impl IntoVar<WidgetFn<ContextMenuArgs>>, disabled_only: bool) -> impl UiNode {
    let menu = menu.into_var();
    let mut pop_state = var(PopupState::Closed).read_only();

    match_node(child, move |c, op| match op {
        UiNodeOp::Init => {
            WIDGET.sub_var(&menu).sub_event(&CLICK_EVENT);
        }
        UiNodeOp::Deinit => {
            POPUP.close_var(pop_state.clone());
        }
        UiNodeOp::Event { update } => {
            c.event(update);
            if let Some(args) = CLICK_EVENT.on_unhandled(update) {
                if args.is_context() {
                    let apply = if disabled_only {
                        args.target.interactivity().is_disabled()
                    } else {
                        args.target.interactivity().is_enabled()
                    };
                    if apply {
                        let menu = menu.get()(ContextMenuArgs { disabled: disabled_only });
                        pop_state = POPUP.open_config(menu, CONTEXT_MENU_ANCHOR_VAR, crate::widgets::popup::CONTEXT_CAPTURE_VAR.get());
                    }
                }
            }
        }
        _ => {}
    })
}

/// Set the position of the context-menu widgets opened for the widget or its descendants.
///
/// Context-menus are inserted as [`POPUP`] when shown, this property defines how the tip layer
/// is aligned with the *anchor* widget, or the cursor.
///
/// By default tips are aligned below the cursor position at the time they are opened.
///
/// This property sets the [`CONTEXT_MENU_ANCHOR_VAR`].
#[property(CONTEXT, default(CONTEXT_MENU_ANCHOR_VAR))]
pub fn context_menu_anchor(child: impl UiNode, mode: impl IntoVar<AnchorMode>) -> impl UiNode {
    with_context_var(child, CONTEXT_MENU_ANCHOR_VAR, mode)
}

/// Context menu popup.
///
/// This widget can be set in [`context_menu`] to define a popup menu that shows when the widget receives
/// a context click.
///
/// [`context_menu`]: fn@context_menu
#[widget($crate::widgets::menu::ContextMenu {
    ($children:expr) => {
        children = $children;
    }
})]
pub struct ContextMenu(popup::SubMenuPopup);
impl ContextMenu {
    fn widget_intrinsic(&mut self) {
        // TODO parent SubMenuPopup requires a `parent_id`, we don't have this here.
    }
}

/// Arguments for context menu widget functions.
pub struct ContextMenuArgs {
    /// Is `true` if the tooltip is for [`disabled_context_menu_fn`], is `false` for [`context_menu_fn`].
    ///
    /// [`context_menu_fn`]: fn@context_menu_fn
    /// [`disabled_context_menu_fn`]: fn@disabled_context_menu_fn
    pub disabled: bool,
}

context_var! {
    /// Position of the context widget in relation to the anchor widget.
    ///
    /// By default the context widget is shown at the cursor.
    pub static CONTEXT_MENU_ANCHOR_VAR: AnchorMode = AnchorMode::context_menu();
}
