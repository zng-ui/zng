//! Context menu widget and properties.

use crate::prelude::rule_line::hr;
use crate::prelude::{button, new_property::*, new_widget::*, toggle, AnchorMode};
use crate::widgets::menu::popup;
use crate::widgets::popup::{PopupState, POPUP};

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
            POPUP.close_var(&pop_state);
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
                        let is_shortcut = args.is_from_keyboard();
                        pop_state = POPUP.open_config(
                            menu,
                            CONTEXT_MENU_ANCHOR_VAR.map_ref(move |(c, s)| if is_shortcut { s } else { c }),
                            crate::widgets::popup::CONTEXT_CAPTURE_VAR.get(),
                        );
                    }
                }
            }
        }
        _ => {}
    })
}

/// Set the position of the context-menu widgets opened for the widget or its descendants.
///
/// This property defines two positions, `(click, shortcut)`, the first is used for context clicks
/// from a pointer device, the second is used for context clicks from keyboard shortcut.
///
/// By default tips are aligned to cursor position at the time they are opened or the top for shortcut.
///
/// This property sets the [`CONTEXT_MENU_ANCHOR_VAR`].
#[property(CONTEXT, default(CONTEXT_MENU_ANCHOR_VAR))]
pub fn context_menu_anchor(child: impl UiNode, click_shortcut: impl IntoVar<(AnchorMode, AnchorMode)>) -> impl UiNode {
    with_context_var(child, CONTEXT_MENU_ANCHOR_VAR, click_shortcut)
}

/// Context menu popup.
///
/// This widget can be set in [`context_menu`] to define a popup menu that shows when the widget receives
/// a context click.
///
/// [`context_menu`]: fn@context_menu
#[widget($crate::widgets::menu::context::ContextMenu {
    ($children:expr) => {
        children = $children;
    }
})]
pub struct ContextMenu(crate::widgets::menu::popup::SubMenuPopup);
impl ContextMenu {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            alt_focus_scope = true;
            style_fn = STYLE_VAR;
        }
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
    pub static CONTEXT_MENU_ANCHOR_VAR: (AnchorMode, AnchorMode) = (AnchorMode::context_menu(), AnchorMode::context_menu_shortcut());

    /// Defines the layout widget for [`ContextMenu!`].
    ///
    /// Is [`popup::default_panel_fn`] by default.
    ///
    /// [`ContextMenu!`]: struct@ContextMenu
    pub static PANEL_FN_VAR: WidgetFn<panel::PanelArgs> = WidgetFn::new(popup::default_panel_fn);

    /// Context-menu popup style in a context.
    ///
    /// Is the [`DefaultStyle!`] by default.
    ///
    /// [`DefaultStyle!`]: struct@DefaultStyle
    pub static STYLE_VAR: StyleFn = StyleFn::new(|_| DefaultStyle!());
}

/// Widget function that generates the context menu layout.
///
/// This property sets [`PANEL_FN_VAR`].
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(ContextMenu))]
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

/// Context menu popup default style.
#[widget($crate::widgets::menu::context::DefaultStyle)]
pub struct DefaultStyle(crate::widgets::menu::popup::DefaultStyle);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            button::replace_style = style_fn!(|_| super::ButtonStyle!());
            toggle::replace_style = style_fn!(|_| super::ToggleStyle!());
            hr::color = button::color_scheme_hovered(button::BASE_COLORS_VAR);
            crate::widgets::icon::ico_size = 18;
        }
    }
}
