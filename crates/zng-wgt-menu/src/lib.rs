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
use zng_wgt::{base_color, margin, prelude::*};
use zng_wgt_access::{AccessRole, access_role};
use zng_wgt_button::BUTTON;
use zng_wgt_container::{child_end, padding};
use zng_wgt_input::focus::{FocusClickBehavior, alt_focus_scope, focus_click_behavior};
use zng_wgt_style::{Style, StyleMix, impl_style_fn, style_fn};

pub mod context;
pub mod popup;
pub mod sub;

/// Menu root panel.
#[widget($crate::Menu { ($children:expr) => { children = $children; } })]
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
            zng_wgt_rule_line::collapse_scope = true;
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
pub fn panel_fn(child: impl IntoUiNode, panel: impl IntoVar<WidgetFn<zng_wgt_panel::PanelArgs>>) -> UiNode {
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
            zng_wgt_rule_line::vr::color = BASE_COLOR_VAR.shade(1);
            zng_wgt_rule_line::vr::height = 1.em();
            zng_wgt_text::icon::ico_size = 18;
        }
    }
}

/// Style applied to all [`Button!`] widgets inside [`Menu!`] root.
///
/// Gives the button a *toolbar-item* look.
///
/// See also [`sub::ButtonStyle!`] for the style of buttons inside the sub-menus.
///
/// [`Button!`]: struct@zng_wgt_button::Button
/// [`Menu!`]: struct@Menu
/// [`sub::ButtonStyle!`]: struct@sub::ButtonStyle
#[widget($crate::ButtonStyle)]
pub struct ButtonStyle(zng_wgt_button::LightStyle);
impl ButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            padding = 4;

            access_role = AccessRole::MenuItem;
            focus_click_behavior = FocusClickBehavior::ExitEnabled;

            zng_wgt_container::child_start =
                BUTTON
                    .cmd()
                    .flat_map(|c| c.as_ref().map(|c| c.icon()).unwrap_or_else(|| const_var(WidgetFn::nil())))
                    .present_data(()),
                0,
            ;
        }
    }
}

/// Alternate style for buttons inside a menu.
///
/// If the button has a command, show the icon as child, if the command has no icon shows the name.
///
/// [`Button!`]: struct@zng_wgt_button::Button
/// [`Menu!`]: struct@Menu
/// [`sub::ButtonStyle!`]: struct@sub::ButtonStyle
#[widget($crate::IconButtonStyle)]
pub struct IconButtonStyle(zng_wgt_button::LightStyle);
impl IconButtonStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            padding = 4;

            access_role = AccessRole::MenuItem;
            focus_click_behavior = FocusClickBehavior::ExitEnabled;

            zng_wgt_container::child =
                BUTTON
                    .cmd()
                    .flat_map(|c| match c {
                        Some(c) => expr_var! {
                            let icon = #{c.icon()};
                            let name = #{c.name()};
                            wgt_fn!(icon, name, |args| {
                                let icon = icon(args);
                                if icon.is_nil() { zng_wgt_text::Text!(name.clone()) } else { icon }
                            })
                        },
                        None => const_var(WidgetFn::nil()),
                    })
                    .present_data(()),
            ;

            zng_wgt_button::cmd_tooltip_fn = wgt_fn!(|args: zng_wgt_button::CmdTooltipArgs| {
                let name = args.cmd.name();
                let info = args.cmd.info();
                let shortcut = args.cmd.shortcut();
                zng_wgt_tooltip::Tip!(zng_wgt_stack::Stack! {
                    direction = zng_wgt_stack::StackDirection::top_to_bottom();
                    spacing = 5;
                    children = ui_vec![
                        zng_wgt_text::Text!(name),
                        zng_wgt_text::Text! {
                            zng_wgt::visibility = info.map(|s| (!s.is_empty()).into());
                            txt = info;
                        },
                        zng_wgt_shortcut::ShortcutText!(shortcut)
                    ];
                })
            });
        }
    }
}

/// Style applied to all [`Toggle!`] widgets inside [`Menu!`] root.
///
/// Gives the toggle a *toolbar-item* look.
///
/// [`Toggle!`]: struct@zng_wgt_toggle::Toggle
/// [`Menu!`]: struct@Menu
#[widget($crate::ToggleStyle)]
pub struct ToggleStyle(zng_wgt_toggle::LightStyle);
impl ToggleStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            padding = 4;
            access_role = AccessRole::MenuItem;
            focus_click_behavior = FocusClickBehavior::ExitEnabled;
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
pub fn icon(child: impl IntoUiNode, icon: impl IntoUiNode) -> UiNode {
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
pub fn icon_fn(child: impl IntoUiNode, icon: impl IntoVar<WidgetFn<()>>) -> UiNode {
    sub::start_column_fn(child, icon)
}

/// Menu item shortcut text.
///
/// Set this on a [`Button!`] inside a sub-menu to define the shortcut text.
///
/// Note that this does not define the click shortcut, just the display of it. The [`ShortcutText!`]
/// widget is recommended.
///
/// [`Button!`]: struct@zng_wgt_button::Button
/// [`ShortcutText!`]: struct@zng_wgt_shortcut::ShortcutText
#[property(CHILD_CONTEXT)]
pub fn shortcut_txt(child: impl IntoUiNode, shortcut: impl IntoUiNode) -> UiNode {
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
pub fn shortcut_spacing(child: impl IntoUiNode, spacing: impl IntoVar<Length>) -> UiNode {
    with_context_var(child, SHORTCUT_SPACING_VAR, spacing)
}
