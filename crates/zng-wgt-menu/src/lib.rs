#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/main/examples/image/res/zng-logo.png")]
//!
//! Menu widgets and properties.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

zng_wgt::enable_widget_macros!();

use colors::BASE_COLOR_VAR;
use zng_ext_font::FontNames;
use zng_ext_input::{focus::FOCUS, mouse::ClickMode};
use zng_ext_l10n::lang;
use zng_wgt::{align, base_color, is_disabled, is_mobile, margin, prelude::*};
use zng_wgt_access::{AccessRole, access_role};
use zng_wgt_button::BUTTON;
use zng_wgt_container::{child_align, child_end, padding};
use zng_wgt_fill::{background_color, foreground_highlight};
use zng_wgt_filter::{opacity, saturate};
use zng_wgt_input::{CursorIcon, cursor, focus::alt_focus_scope};
use zng_wgt_input::{click_mode, focus::is_focused, mouse::on_pre_mouse_enter};
use zng_wgt_size_offset::size;
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};
use zng_wgt_text::Text;

pub mod context;
pub mod popup;
pub mod sub;

/// Menu root panel.
#[widget($crate::Menu {
    ($children:expr) => {
        children = $children;
    }
})]
pub struct Menu(StyleMix<zng_wgt_panel::Panel>);
impl Menu {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;
            alt_focus_scope = true;
            zng_wgt_panel::panel_fn = PANEL_FN_VAR;
            style_base_fn = style_fn!(|_| DefaultStyle!());
            access_role = AccessRole::Menu;
        }
    }
}
impl_style_fn!(Menu);

context_var! {
    /// Defines the layout widget for [`Menu!`].
    ///
    /// Is a [`Wrap!`] panel by default.
    ///
    /// [`Menu!`]: struct@Menu
    /// [`Wrap!`]: struct@zng_wgt_wrap::Wrap
    pub static PANEL_FN_VAR: WidgetFn<zng_wgt_panel::PanelArgs> = wgt_fn!(|a: zng_wgt_panel::PanelArgs| {
        zng_wgt_wrap::Wrap! {
            children = a.children;
        }
    });

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
pub fn panel_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<zng_wgt_panel::PanelArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Default [`Menu!`] style.
///
/// Gives the button a *menu-item* look.
///
/// [`Menu!`]: struct@Menu
#[widget($crate::DefaultStyle)]
pub struct DefaultStyle(Style);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            // also see context::DefaultStyle

            base_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));
            zng_wgt_button::style_fn = style_fn!(|_| ButtonStyle!());
            zng_wgt_toggle::style_fn = style_fn!(|_| ToggleStyle!());
            zng_wgt_rule_line::hr::color = BASE_COLOR_VAR.shade(1);
            zng_wgt_text::icon::ico_size = 18;
        }
    }
}

/// Style applied to all [`Button!`] widgets inside [`Menu!`].
///
/// Gives the button a *menu-item* look.
///
/// [`Button!`]: struct@zng_wgt_button::Button
/// [`Menu!`]: struct@Menu
#[widget($crate::ButtonStyle)]
pub struct ButtonStyle(Style);
impl ButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            replace = true;

            sub::column_width_padding = true;
            padding = (4, 0);
            child_align = Align::START;

            base_color = light_dark(rgb(0.82, 0.82, 0.82), rgb(0.18, 0.18, 0.18));
            background_color = BASE_COLOR_VAR.rgba();
            opacity = 90.pct();
            foreground_highlight = unset!;
            zng_wgt_tooltip::tooltip_fn = WidgetFn::nil(); // cmd sets tooltip

            click_mode = ClickMode::release();// part of press-and-drag to click (see SubMenuPopup)

            access_role = AccessRole::MenuItem;

            on_pre_mouse_enter = hn!(|_| {
                FOCUS.focus_widget(WIDGET.id(), false);
            });

            shortcut_txt = Text! {
                txt = BUTTON.cmd().flat_map(|c| match c {
                    Some(c) => c.shortcut_txt(),
                    None => const_var(Txt::from(""))
                });
                align = Align::CENTER;
            };

            icon_fn = BUTTON.cmd().flat_map(|c| match c {
                Some(c) => c.icon(),
                None => const_var(WidgetFn::nil())
            });

            when *#is_focused {
                background_color = BASE_COLOR_VAR.shade(1);
                opacity = 100.pct();
            }

            when *#is_disabled {
                saturate = false;
                opacity = 50.pct();
                cursor = CursorIcon::NotAllowed;
            }

            when *#is_mobile {
                shortcut_txt = NilUiNode;
            }
        }
    }
}

/// Command button for touch.
///
/// This a menu button style that has a `cmd` property, it changes the visibility to collapse when the command
/// is disabled.
#[widget($crate::TouchButtonStyle)]
pub struct TouchButtonStyle(Style);
impl TouchButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            zng_wgt::corner_radius = 0;
            zng_wgt::visibility = BUTTON
                .cmd()
                .flat_map(|c| match c {
                    Some(c) => c.is_enabled(),
                    None => const_var(true),
                })
                .map_into();
        }
    }
}

/// Style applied to all [`Button!`] widgets inside [`Menu!`].
///
/// Gives the toggle a *menu-item* look, the check mark is placed in the icon position.
///
/// [`Button!`]: struct@zng_wgt_button::Button
/// [`Menu!`]: struct@Menu
#[widget($crate::ToggleStyle)]
pub struct ToggleStyle(ButtonStyle);
impl ToggleStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            replace = true;

            click_mode = ClickMode::release();
            access_role = AccessRole::MenuItemCheckBox;

            sub::start_column_fn = wgt_fn!(|_| Text! {
                size = 1.2.em();
                font_family = FontNames::system_ui(&lang!(und));
                align = Align::CENTER;

                txt = "✓";
                when #{zng_wgt_toggle::IS_CHECKED_VAR}.is_none() {
                    txt = "━";
                }

                font_color = zng_wgt_text::FONT_COLOR_VAR.map(|c| c.transparent());
                when #{zng_wgt_toggle::IS_CHECKED_VAR}.unwrap_or(true) {
                    font_color = zng_wgt_text::FONT_COLOR_VAR;
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
/// [`Button!`]: struct@zng_wgt_button::Button
/// [`Icon!`]: struct@zng_wgt_text::icon::Icon
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
/// [`Button!`]: struct@zng_wgt_button::Button
/// [`Icon!`]: struct@zng_wgt_text::icon::Icon
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
/// [`Button!`]: struct@zng_wgt_button::Button
#[property(CHILD_CONTEXT)]
pub fn shortcut_txt(child: impl UiNode, shortcut: impl UiNode) -> impl UiNode {
    let shortcut = margin(shortcut, sub::END_COLUMN_WIDTH_VAR.map(|w| SideOffsets::new(0, w.clone(), 0, 0)));
    child_end(child, shortcut, SHORTCUT_SPACING_VAR)
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
