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
pub(super) fn inspect_node(child: impl crate::core::UiNode, can_inspect: impl crate::core::var::IntoVar<bool>) -> impl crate::core::UiNode {
    use crate::core::inspector::prompt::{write_tree, WriteTreeState};
    use crate::core::{handler::hn, task};

    let mut state = WriteTreeState::none();

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
            write_tree(ctx.vars, ctx.info_tree, &state, &mut buffer);

            state = WriteTreeState::new(ctx.info_tree);

            task::spawn_wait(move || {
                use std::io::*;
                stdout()
                    .write_all(&buffer)
                    .unwrap_or_else(|e| tracing::error!("error printing frame {e}"));
            });
        }),
    )
}
