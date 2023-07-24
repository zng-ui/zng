//! Menu widgets and properties.
//!

use crate::prelude::{button, new_widget::*, toggle};

/// Menu root panel.
#[widget($crate::widgets::menu::Menu)]
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
            column_width_padding = true;
            padding = (4, 0);
            child_align = Align::START;

            background_color = color_scheme_pair(button::BASE_COLORS_VAR);
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

/// Style applied to all [`Button!`] widgets inside [`Menu!`].
///
/// Gives the toggle a *menu-item* look, the checkmark is placed in the icon position.
#[widget($crate::widgets::menu::ToggleStyle)]
pub struct ToggleStyle(ButtonStyle);
impl ToggleStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            icon = crate::widgets::Text! {
                // size = 1.2.em();
                font_family = FontNames::system_ui(&lang!(und));
                txt_align = Align::CENTER;
                align = Align::CENTER;

                txt = "✓";
                when #{toggle::IS_CHECKED_VAR}.is_none() {
                    txt = "━";
                }

                font_color = text::FONT_COLOR_VAR.map(|c| c.transparent());
                when #{toggle::IS_CHECKED_VAR}.unwrap_or(true) {
                    font_color = text::FONT_COLOR_VAR;
                }
            }
        }
    }
}

/// Submenu parent.
#[widget($crate::widgets::menu::SubMenu)]
pub struct SubMenu(WidgetBase);
impl SubMenu {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
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

    /// Width of the icon/checkmark column.
    pub static START_COLUMN_WIDTH_VAR: Length = 42;

    /// Width of the sub-menu expand symbol column.
    pub static END_COLUMN_WIDTH_VAR: Length = 42;
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

/// Width of the icon/checkmark column.
///
/// This property sets [`START_COLUMN_WIDTH_VAR`].
#[property(CONTEXT, default(START_COLUMN_WIDTH_VAR), widget_impl(Menu))]
pub fn start_column_width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, START_COLUMN_WIDTH_VAR, width)
}

/// Width of the sub-menu expand symbol column.
///
/// This property sets [`END_COLUMN_WIDTH_VAR`].
#[property(CONTEXT, default(END_COLUMN_WIDTH_VAR), widget_impl(Menu))]
pub fn end_column_width(child: impl UiNode, width: impl IntoVar<Length>) -> impl UiNode {
    with_context_var(child, END_COLUMN_WIDTH_VAR, width)
}

/// Sets the icon of a button inside the menu.
#[property(FILL)]
pub fn icon(child: impl UiNode, icon: impl UiNode) -> impl UiNode {
    let icon = align(icon, Align::START);
    let icon = margin(icon, 4);
    background(child, icon)
}
