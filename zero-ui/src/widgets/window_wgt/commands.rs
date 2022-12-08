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
    use zero_ui_core::inspector::WidgetInfoInspectorExt;

    use crate::core::{
        handler::{clone_move, hn},
        inspector::prompt::WriteTreeState,
        text::{formatx, Text},
        units::Align,
        var::{var, var_from, Var},
        widget_builder::property_id,
        window::{WindowId, Windows},
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

            let mut buffer = vec![];
            inspector_state.write_update(ctx.info_tree, &mut buffer);

            let txt = String::from_utf8_lossy(&buffer).into_owned();
            inspector_text.set_ne(ctx, txt);

            let parent = ctx.path.window_id();
            Windows::req(ctx).focus_or_open(
                inspector_id,
                clone_move!(inspector_text, |ctx| {
                    let tree = Windows::req(ctx.services).widget_tree(parent);
                    let title = if let Some(title) = tree.unwrap().root().inspect_property(property_id!(crate::widgets::window::title)) {
                        title.downcast_var::<Text>(0).map(|t| formatx!("{t} - Inspector")).boxed()
                    } else {
                        var_from("Inspector").boxed()
                    };

                    crate::widgets::window! {
                        parent;
                        title;
                        child = crate::widgets::scroll! {
                            child = crate::widgets::ansi_text! { txt = inspector_text; };
                            child_align = Align::TOP_LEFT;
                            padding = 5;
                        }
                    }
                }),
            );

            // crate::core::task::spawn_wait(move || {
            //     use std::io::*;
            //     stdout()
            //         .write_all(&buffer)
            //         .unwrap_or_else(|e| tracing::error!("error printing frame {e}"));
            // });
        }),
    )
}
