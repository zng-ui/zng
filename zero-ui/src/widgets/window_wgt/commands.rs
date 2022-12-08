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
    use zero_ui_core::{units::Align, window::Windows};

    use crate::core::handler::hn;
    use crate::core::inspector::prompt::WriteTreeState;

    let mut state = WriteTreeState::new();

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
            state.write_update(ctx.info_tree, &mut buffer);

            let txt = String::from_utf8_lossy(&buffer).into_owned();
            let parent = ctx.path.window_id();
            Windows::req(ctx.services).open(move |_| {
                crate::widgets::window! {
                    parent;
                    title = "Inspector";
                    child = crate::widgets::scroll! {
                        child = crate::widgets::ansi_text! { txt; };
                        child_align = Align::TOP_LEFT;
                        padding = 5;
                    }
                }
            });

            //task::spawn_wait(move || {
            //    use std::io::*;
            //    stdout()
            //        .write_all(&buffer)
            //        .unwrap_or_else(|e| tracing::error!("error printing frame {e}"));
            //});
        }),
    )
}
