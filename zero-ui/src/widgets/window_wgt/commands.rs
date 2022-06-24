//! Commands that control the scoped window.

use crate::core::{command::*, gesture::*};

pub use crate::core::window::commands::*;

command! {
    /// Represent the window **inspect** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Debug Inspector"                                     |
    /// | [`info`]     | "Inspect the current window."                         |
    /// | [`shortcut`] | `CTRL|SHIFT+I`, `F12`                                 |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub InspectCommand
        .init_name("Debug Inspector")
        .init_info("Inspect the current window.")
        .init_shortcut([shortcut!(CTRL|SHIFT+I), shortcut!(F12)]);
}

#[cfg(inspector)]
pub(super) fn inspect_node(child: impl crate::core::UiNode, can_inspect: impl crate::core::var::IntoVar<bool>) -> impl crate::core::UiNode {
    use crate::core::inspector::{write_tree, WriteTreeState};
    use crate::core::{handler::hn, task};

    let mut state = WriteTreeState::none();

    let can_inspect = can_inspect.into_var();

    on_command(
        child,
        |ctx| InspectCommand.scoped(ctx.path.window_id()),
        move |_| can_inspect.clone(),
        hn!(|ctx, args: &CommandArgs| {
            if !args.enabled {
                return;
            }

            args.propagation().stop();

            let mut buffer = vec![];
            write_tree(ctx.info_tree, &state, &mut buffer);

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
