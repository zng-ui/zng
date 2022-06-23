//! Commands that control the scoped window.

use crate::{
    command::*,
    context::WindowContext,
    event::{EventUpdateArgs, Events},
    gesture::*,
    var::*,
};

use super::{WindowId, WindowState, WindowVars, WindowsExt};

command! {
    /// Represents the window **close** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Close Window"                                        |
    /// | [`info`]     | "Close the current window."                           |
    /// | [`shortcut`] | `ALT+F4`, `CTRL+W`                                    |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub CloseCommand
        .init_name("Close")
        .init_info("Close the current window.")
        .init_shortcut([shortcut!(ALT+F4), shortcut!(CTRL+W)]);

    /// Represents the window **minimize** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Minimize Window"                                     |
    /// | [`info`]     | "Minimize the current window."                        |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    pub MinimizeCommand
        .init_name("Minimize")
        .init_info("Minimize the current window.");

    /// Represents the window **maximize** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Maximize Window"                                     |
    /// | [`info`]     | "Maximize the current window."                        |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    pub MaximizeCommand
        .init_name("Maximize")
        .init_info("Maximize the current window.");

    /// Represents the window **toggle fullscreen** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Full-Screen"                                         |
    /// | [`info`]     | "Toggle full-screen mode on the current window."      |
    /// | [`shortcut`] | `CMD|SHIFT+F` on MacOS, `F11` on other systems.       |
    ///
    /// # Behavior
    ///
    /// This command is about the *windowed* fullscreen state ([`WindowState::Fullscreen`]),
    /// use the [`ExclusiveFullscreenCommand`] to toggle *exclusive* video mode fullscreen.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FullscreenCommand
        .init_name("Full-Screen")
        .init_info("Toggle full-screen mode on the current window.")
        .init_shortcut({
            if cfg!(target_os = "macos") {
                shortcut!(CTRL|SHIFT+F)
            } else {
                shortcut!(F11)
            }
        });

    /// Represents the window **toggle fullscreen** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Minimize Window"                                     |
    /// | [`info`]     | "Minimize the current window."                        |
    ///
    /// # Behavior
    ///
    /// This command is about the *exclusive* fullscreen state ([`WindowState::Exclusive`]),
    /// use the [`FullscreenCommand`] to toggle *windowed* fullscreen.
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    pub ExclusiveFullscreenCommand
        .init_name("Exclusive Full-Screen")
        .init_info("Toggle exclusive full-screen mode on the current window.");

    /// Represents the window **restore** action.
    ///
    /// Restores the window to its previous not-minimized state or normal state.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                                      |
    /// |--------------|----------------------------------------------------------------------------|
    /// | [`name`]     | "Restore Window"                                                           |
    /// | [`info`]     | "Restores the window to its previous not-minimized state or normal state." |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    pub RestoreCommand
        .init_name("Restore")
        .init_info("Restores the window to its previous not-minimized state or normal state.");
}

pub(super) struct WindowCommands {
    maximize_handle: CommandHandle,
    minimize_handle: CommandHandle,
    restore_handle: CommandHandle,

    fullscreen_handle: CommandHandle,
    exclusive_handle: CommandHandle,

    close_handle: CommandHandle,
}
impl WindowCommands {
    pub fn new(window_id: WindowId, events: &mut Events) -> Self {
        WindowCommands {
            maximize_handle: MaximizeCommand.scoped(window_id).new_handle(events, false),
            minimize_handle: MinimizeCommand.scoped(window_id).new_handle(events, false),
            restore_handle: RestoreCommand.scoped(window_id).new_handle(events, false),
            fullscreen_handle: FullscreenCommand.scoped(window_id).new_handle(events, true),
            exclusive_handle: ExclusiveFullscreenCommand.scoped(window_id).new_handle(events, true),

            close_handle: CloseCommand.scoped(window_id).new_handle(events, true),
        }
    }

    pub fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WindowContext, window_vars: &WindowVars, args: &A) {
        let scope = *ctx.window_id;
        if let Some(args) = MaximizeCommand.scoped(scope).update(args) {
            args.handle_enabled(&self.maximize_handle, |_| {
                window_vars.state().set_ne(ctx, WindowState::Maximized);
            });
        } else if let Some(args) = MinimizeCommand.scoped(scope).update(args) {
            args.handle_enabled(&self.minimize_handle, |_| {
                window_vars.state().set_ne(ctx, WindowState::Minimized);
            });
        } else if let Some(args) = RestoreCommand.scoped(scope).update(args) {
            args.handle_enabled(&self.restore_handle, |_| {
                window_vars.state().set_ne(ctx.vars, window_vars.restore_state().copy(ctx.vars));
            });
        } else if let Some(args) = CloseCommand.scoped(scope).update(args) {
            args.handle_enabled(&self.close_handle, |_| {
                let _ = ctx.services.windows().close(scope);
            });
        } else if let Some(args) = FullscreenCommand.scoped(scope).update(args) {
            args.handle_enabled(&self.fullscreen_handle, |_| {
                if let WindowState::Fullscreen = window_vars.state().copy(ctx) {
                    window_vars.state().set(ctx.vars, window_vars.restore_state().copy(ctx.vars));
                } else {
                    window_vars.state().set(ctx.vars, WindowState::Fullscreen);
                }
            });
        } else if let Some(args) = ExclusiveFullscreenCommand.scoped(scope).update(args) {
            args.handle_enabled(&self.exclusive_handle, |_| {
                if let WindowState::Exclusive = window_vars.state().copy(ctx) {
                    window_vars.state().set(ctx.vars, window_vars.restore_state().copy(ctx.vars));
                } else {
                    window_vars.state().set(ctx, WindowState::Exclusive);
                }
            });
        }
    }

    pub fn init(&mut self, vars: &Vars, window_vars: &WindowVars) {
        self.update_state(window_vars.state().copy(vars));
    }

    pub fn update(&mut self, vars: &Vars, window_vars: &WindowVars) {
        if let Some(state) = window_vars.state().copy_new(vars) {
            self.update_state(state);
        }
    }

    fn update_state(&mut self, state: WindowState) {
        self.restore_handle.set_enabled(state != WindowState::Normal);
        self.maximize_handle.set_enabled(state != WindowState::Maximized);
        self.minimize_handle.set_enabled(state != WindowState::Minimized);
    }
}
