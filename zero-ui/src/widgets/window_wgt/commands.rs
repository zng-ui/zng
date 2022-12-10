//! Commands that control the scoped window.

use crate::core::{event::*, gesture::*};

pub use crate::core::window::commands::*;

command! {
    /// Represent the window **inspect** action.
    pub static INSPECT_CMD = {
        name: "Debug Inspector",
        info: "Inspect the current window.",
        shortcut: [shortcut!(CTRL|SHIFT+I), shortcut!(F12)],
    };
}

#[cfg(inspector)]
pub(super) fn inspect_node(
    child: impl crate::core::widget_instance::UiNode,
    can_inspect: impl crate::core::var::IntoVar<bool>,
) -> impl crate::core::widget_instance::UiNode {
    use zero_ui_core::window::{WindowIcon, WindowVars};

    use crate::{
        core::{
            color::ColorScheme,
            handler::{clone_move, hn},
            inspector::{prompt::WriteTreeState, WidgetInfoInspectorExt},
            text::{formatx, Text},
            units::Align,
            var::{var, var_from, Var},
            widget_builder::property_id,
            window::{WindowId, Windows},
        },
        widgets::scroll::ScrollMode,
    };

    let mut inspector_state = WriteTreeState::new();
    let inspector_id = WindowId::new_unique();
    let inspector_text = var(Text::empty());

    let can_inspect = can_inspect.into_var();

    on_command(
        child,
        |ctx| INSPECT_CMD.scoped(ctx.path.window_id()),
        move |_| can_inspect.clone(),
        hn!(|ctx, args: &CommandArgs| {
            if !args.enabled {
                return;
            }

            args.propagation().stop();

            let txt = inspector_state.ansi_string_update(ctx.info_tree);
            inspector_text.set_ne(ctx, txt);

            let inspected = ctx.path.window_id();
            let parent = WindowVars::req(ctx).parent().get().unwrap_or(inspected);
            Windows::req(ctx).focus_or_open(
                inspector_id,
                clone_move!(inspector_text, |ctx| {
                    use crate::widgets::*;

                    let tree = Windows::req(ctx.services).widget_tree(inspected);
                    let title = if let Some(title) = tree.unwrap().root().inspect_property(property_id!(window::title)) {
                        title.downcast_var::<Text>(0).map(|t| formatx!("{t} - Inspector")).boxed()
                    } else {
                        var_from("Inspector").boxed()
                    };
                    let icon = if let Some(icon) = tree.unwrap().root().inspect_property(property_id!(window::icon)) {
                        icon.downcast_var::<WindowIcon>(0).clone().boxed()
                    } else {
                        var(WindowIcon::Default).boxed()
                    };

                    window! {
                        parent;
                        title;
                        icon;
                        color_scheme = ColorScheme::Dark;
                        child = scroll! {
                            child = ansi_text! { txt = inspector_text; };
                            mode = ScrollMode::VERTICAL;
                            child_align = Align::TOP_LEFT;
                            padding = 5;
                        }
                    }
                }),
            );
        }),
    )
}
