//! Menu widgets and properties.

#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zero_ui_wgt::enable_widget_macros!();

use zero_ui_ext_font::FontNames;
use zero_ui_ext_input::{focus::FOCUS, gesture::ClickArgs, mouse::ClickMode};
use zero_ui_ext_l10n::lang;
use zero_ui_wgt::{align, is_disabled, margin, prelude::*};
use zero_ui_wgt_access::{access_role, AccessRole};
use zero_ui_wgt_button::Button;
use zero_ui_wgt_container::{child_align, child_insert_end, padding};
use zero_ui_wgt_fill::{background_color, foreground_highlight};
use zero_ui_wgt_filters::{opacity, saturate};
use zero_ui_wgt_input::{click_mode, focus::is_focused, mouse::on_pre_mouse_enter};
use zero_ui_wgt_input::{cursor, focus::alt_focus_scope, CursorIcon};
use zero_ui_wgt_size_offset::size;
use zero_ui_wgt_style::{style_fn, Style, StyleFn, StyleMix};
use zero_ui_wgt_text::Text;
use zero_ui_wgt_view::{wgt_fn, WidgetFn};

pub mod context;
pub mod popup;
pub mod sub;

/// Menu root panel.
#[widget($crate::Menu {
    ($children:expr) => {
        children = $children;
    }
})]
pub struct Menu(StyleMix<zero_ui_wgt_panel::Panel>);
impl Menu {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            alt_focus_scope = true;
            zero_ui_wgt_panel::panel_fn = PANEL_FN_VAR;
            style_fn = STYLE_VAR;
            access_role = AccessRole::Menu;
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
    pub static PANEL_FN_VAR: WidgetFn<zero_ui_wgt_panel::PanelArgs> = wgt_fn!(|a: zero_ui_wgt_panel::PanelArgs| {
        zero_ui_wgt_wrap::Wrap! {
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
pub fn panel_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<zero_ui_wgt_panel::PanelArgs>>) -> impl UiNode {
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
    zero_ui_wgt_style::with_style_extension(child, STYLE_VAR, style)
}

/// Default [`Menu!`] style.
///
/// Gives the button a *menu-item* look.
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            // also see context::DefaultStyle

            zero_ui_wgt_button::replace_style = style_fn!(|_| ButtonStyle!());
            zero_ui_wgt_toggle::replace_style = style_fn!(|_| ToggleStyle!());
            zero_ui_wgt_rule_line::hr::color = zero_ui_wgt_button::color_scheme_hovered(zero_ui_wgt_button::BASE_COLORS_VAR);
            zero_ui_wgt_text::icon::ico_size = 18;
        }
    }
}

/// Style applied to all [`Button!`] widgets inside [`Menu!`].
///
/// Gives the button a *menu-item* look.
#[widget($crate::ButtonStyle)]
pub struct ButtonStyle(Style);
impl ButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            sub::column_width_padding = true;
            padding = (4, 0);
            child_align = Align::START;

            background_color = color_scheme_pair(zero_ui_wgt_button::BASE_COLORS_VAR);
            opacity = 90.pct();
            foreground_highlight = unset!;

            click_mode = ClickMode::release();// part of press-and-drag to click (see SubMenuPopup)

            access_role = AccessRole::MenuItem;

            on_pre_mouse_enter = hn!(|_| {
                FOCUS.focus_widget(WIDGET.id(), false);
            });

            when *#is_focused {
                background_color = zero_ui_wgt_button::color_scheme_hovered(zero_ui_wgt_button::BASE_COLORS_VAR);
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
#[widget($crate::ToggleStyle)]
pub struct ToggleStyle(ButtonStyle);
impl ToggleStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            click_mode = ClickMode::release();
            access_role = AccessRole::MenuItemCheckBox;

            sub::start_column_fn = wgt_fn!(|_ | Text! {
                size = 1.2.em();
                font_family = FontNames::system_ui(&lang!(und));
                align = Align::CENTER;

                txt = "✓";
                when #{zero_ui_wgt_toggle::IS_CHECKED_VAR}.is_none() {
                    txt = "━";
                }

                font_color = zero_ui_wgt_text::FONT_COLOR_VAR.map(|c| c.transparent());
                when #{zero_ui_wgt_toggle::IS_CHECKED_VAR}.unwrap_or(true) {
                    font_color = zero_ui_wgt_text::FONT_COLOR_VAR;
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
/// [`Button!`]: struct@zero_ui_wgt_button::Button
/// [`Icon!`]: struct@zero_ui_wgt_text::icon::Icon
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
/// [`Button!`]: struct@zero_ui_wgt_button::Button
/// [`Icon!`]: struct@zero_ui_wgt_text::icon::Icon
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
/// [`Button!`]: struct@zero_ui_wgt_button::Button
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
/// are set with values from the command metadata, the `on_click` handle is set to call the command and the `enabled` and
/// `visibility` are set from the command handle status.
#[widget($crate::CmdButton {
    ($cmd:expr) => {
        cmd = $cmd;
    }
})]
pub struct CmdButton(Button);
impl CmdButton {
    /// Build the button from the `cmd` value.
    pub fn widget_build(&mut self) -> impl UiNode {
        use zero_ui_wgt_text::icon::CommandIconExt as _;

        if let Some(cmd) = self.widget_builder().capture_value::<Command>(property_id!(Self::cmd)) {
            widget_set! {
                self;

                enabled = cmd.is_enabled();
                visibility = cmd.has_handlers().map_into();

                child = Text!(cmd.name());

                shortcut_txt = Text! {
                    txt = cmd.shortcut_txt();
                    align = Align::CENTER;
                };
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

/// Command button for touch.
///
/// This a menu button that has a `cmd` property, if the property is set the button `child` is set with value from the
/// command metadata, the `on_click` handle is set to call the command and the `enabled` and `visibility` are set from
/// the command `is_enabled` status.
#[widget($crate::TouchCmdButton {
    ($cmd:expr) => {
        cmd = $cmd;
    }
})]
pub struct TouchCmdButton(Button);
impl TouchCmdButton {
    /// Build the button from the `cmd` value.
    pub fn widget_build(&mut self) -> impl UiNode {
        if let Some(cmd) = self.widget_builder().capture_value::<Command>(property_id!(Self::cmd)) {
            widget_set! {
                self;

                enabled = cmd.is_enabled();
                visibility = cmd.is_enabled().map_into();

                child = Text!(cmd.name());

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

    widget_impl! {
        /// The button command.
        pub cmd(cmd: impl IntoValue<Command>);
    }
}
