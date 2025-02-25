//! Context menu widget and properties.

use colors::BASE_COLOR_VAR;
use zng_ext_input::gesture::CLICK_EVENT;
use zng_wgt::prelude::*;
use zng_wgt_input::focus::alt_focus_scope;
use zng_wgt_layer::{
    AnchorMode,
    popup::{CONTEXT_CAPTURE_VAR, POPUP, PopupState},
};
use zng_wgt_stack::{Stack, StackDirection};
use zng_wgt_style::{impl_style_fn, style_fn};

/// Defines the context menu shown when the widget is enabled and receives a context click.
///
/// The `menu` can be any widget, the [`ContextMenu!`] is recommended. The menu widget is open
/// using [`POPUP`] and is expected to close itself when the context action is finished or it
/// loses focus.
///
/// [`ContextMenu!`]: struct@ContextMenu
/// [`POPUP`]: zng_wgt_layer::popup::POPUP
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
            POPUP.close(&pop_state);
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
                        args.propagation().stop();

                        let menu = menu.get()(ContextMenuArgs {
                            anchor_id: WIDGET.id(),
                            disabled: disabled_only,
                        });
                        let is_shortcut = args.is_from_keyboard();
                        pop_state = POPUP.open_config(
                            menu,
                            CONTEXT_MENU_ANCHOR_VAR.map_ref(move |(c, s)| if is_shortcut { s } else { c }),
                            CONTEXT_CAPTURE_VAR.get(),
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
#[widget($crate::context::ContextMenu {
    ($children:expr) => {
        children = $children;
    }
})]
pub struct ContextMenu(crate::popup::SubMenuPopup);
impl ContextMenu {
    fn widget_intrinsic(&mut self) {
        self.style_intrinsic(STYLE_FN_VAR, property_id!(self::style_fn));
        widget_set! {
            self;
            alt_focus_scope = true;
            style_base_fn = style_fn!(|_| DefaultStyle!());
        }
    }
}
impl_style_fn!(ContextMenu);

/// Arguments for context menu widget functions.
pub struct ContextMenuArgs {
    /// ID of the widget the menu is anchored to.
    pub anchor_id: WidgetId,

    /// Is `true` if the menu is for [`disabled_context_menu_fn`], is `false` for [`context_menu_fn`].
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
    /// [`popup::default_panel_fn`]: crate::popup::default_panel_fn
    pub static PANEL_FN_VAR: WidgetFn<zng_wgt_panel::PanelArgs> = WidgetFn::new(crate::popup::default_panel_fn);
}

/// Widget function that generates the context menu layout.
///
/// This property sets [`PANEL_FN_VAR`].
#[property(CONTEXT, default(PANEL_FN_VAR), widget_impl(ContextMenu))]
pub fn panel_fn(child: impl UiNode, panel: impl IntoVar<WidgetFn<zng_wgt_panel::PanelArgs>>) -> impl UiNode {
    with_context_var(child, PANEL_FN_VAR, panel)
}

/// Context menu popup default style.
#[widget($crate::context::DefaultStyle)]
pub struct DefaultStyle(crate::popup::DefaultStyle);
impl DefaultStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;
            replace = true;
            zng_wgt_button::style_fn = style_fn!(|_| super::ButtonStyle!());
            zng_wgt_toggle::style_fn = style_fn!(|_| super::ToggleStyle!());
            zng_wgt_rule_line::hr::color = BASE_COLOR_VAR.shade(1);
            zng_wgt_text::icon::ico_size = 18;
        }
    }
}

/// Touch context menu popup default style.
#[widget($crate::context::TouchStyle)]
pub struct TouchStyle(crate::popup::DefaultStyle);
impl TouchStyle {
    fn widget_intrinsic(&mut self) {
        widget_set! {
            self;

            panel_fn = wgt_fn!(|args: zng_wgt_panel::PanelArgs| Stack! {
                direction = StackDirection::left_to_right();
                children = args.children;
            });
            zng_wgt_button::style_fn = style_fn!(|_| super::TouchButtonStyle!());
            zng_wgt_rule_line::vr::color = BASE_COLOR_VAR.shade(1);
        }
    }
}
