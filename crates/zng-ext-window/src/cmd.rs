//! Commands that control the scoped window.

use zng_app::{
    event::{command, CommandHandle, CommandInfoExt, CommandNameExt},
    shortcut::{shortcut, CommandShortcutExt},
    update::EventUpdate,
    window::{WindowId, WINDOW},
};
use zng_var::Var;
use zng_view_api::window::WindowState;

use crate::{WindowVars, WINDOWS};

command! {
    /// Represents the window **close** action.
    pub static CLOSE_CMD = {
        name: "Close",
        info: "Close the window.",
        shortcut: [shortcut!(ALT+F4), shortcut!(CTRL+'W')],
    };

    /// Represents the window **minimize** action.
    pub static MINIMIZE_CMD = {
        name: "Minimize",
        info: "Minimize the window.",
    };

    /// Represents the window **maximize** action.
    pub static MAXIMIZE_CMD = {
        name: "Maximize",
        info: "Maximize the window.",
    };

    /// Represents the window **toggle fullscreen** action.
    ///
    /// # Behavior
    ///
    /// This command is about the *windowed* fullscreen state ([`WindowState::Fullscreen`]),
    /// use the [`EXCLUSIVE_FULLSCREEN_CMD`] to toggle *exclusive* video mode fullscreen.
    pub static FULLSCREEN_CMD = {
        name: "Full-Screen",
        info: "Toggle full-screen mode on the window.",
        shortcut: {
            let a = if cfg!(target_os = "macos") {
                shortcut!(CTRL|SHIFT+'F')
            } else {
                shortcut!(F11)
            };
            [a, shortcut!(ZoomToggle)]
        }
    };

    /// Represents the window **toggle fullscreen** action.
    ///
    /// # Behavior
    ///
    /// This command is about the *exclusive* fullscreen state ([`WindowState::Exclusive`]),
    /// use the [`FULLSCREEN_CMD`] to toggle *windowed* fullscreen.
    pub static EXCLUSIVE_FULLSCREEN_CMD = {
        name: "Exclusive Full-Screen",
        info: "Toggle exclusive full-screen mode on the window.",
    };

    /// Represents the window **restore** action.
    ///
    /// Restores the window to its previous non-minimized state or normal state.
    pub static RESTORE_CMD = {
        name: "Restore",
        info: "Restores the window to its previous non-minimized state or normal state.",
    };

    /// Represents the **close IME** action.
    ///
    /// If any IME preview is active close it without committing.
    pub static CANCEL_IME_CMD;

    /// Represents the window **open title bar context menu** action.
    ///
    /// # Parameter
    ///
    /// This command supports an optional parameter, it can be a [`DipPoint`] or [`PxPoint`] that defines
    /// the menu position.
    pub static OPEN_TITLE_BAR_CONTEXT_MENU_CMD;
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
    pub fn new(window_id: WindowId) -> Self {
        WindowCommands {
            maximize_handle: MAXIMIZE_CMD.scoped(window_id).subscribe(false),
            minimize_handle: MINIMIZE_CMD.scoped(window_id).subscribe(false),
            restore_handle: RESTORE_CMD.scoped(window_id).subscribe(false),
            fullscreen_handle: FULLSCREEN_CMD.scoped(window_id).subscribe(true),
            exclusive_handle: EXCLUSIVE_FULLSCREEN_CMD.scoped(window_id).subscribe(true),
            close_handle: CLOSE_CMD.scoped(window_id).subscribe(true),
        }
    }

    pub fn event(&mut self, window_vars: &WindowVars, update: &EventUpdate) {
        let scope = WINDOW.id();
        if let Some(args) = MAXIMIZE_CMD.scoped(scope).on(update) {
            args.handle_enabled(&self.maximize_handle, |_| {
                window_vars.state().set(WindowState::Maximized);
            });
        } else if let Some(args) = MINIMIZE_CMD.scoped(scope).on(update) {
            args.handle_enabled(&self.minimize_handle, |_| {
                window_vars.state().set(WindowState::Minimized);
            });
        } else if let Some(args) = RESTORE_CMD.scoped(scope).on(update) {
            args.handle_enabled(&self.restore_handle, |_| {
                window_vars.state().set(window_vars.restore_state().get());
            });
        } else if let Some(args) = CLOSE_CMD.scoped(scope).on(update) {
            args.handle_enabled(&self.close_handle, |_| {
                let _ = WINDOWS.close(scope);
            });
        } else if let Some(args) = FULLSCREEN_CMD.scoped(scope).on(update) {
            args.handle_enabled(&self.fullscreen_handle, |_| {
                if let WindowState::Fullscreen = window_vars.state().get() {
                    window_vars.state().set(window_vars.restore_state().get());
                } else {
                    window_vars.state().set(WindowState::Fullscreen);
                }
            });
        } else if let Some(args) = EXCLUSIVE_FULLSCREEN_CMD.scoped(scope).on(update) {
            args.handle_enabled(&self.exclusive_handle, |_| {
                if let WindowState::Exclusive = window_vars.state().get() {
                    window_vars.state().set(window_vars.restore_state().get());
                } else {
                    window_vars.state().set(WindowState::Exclusive);
                }
            });
        }
    }

    pub fn init(&mut self, window_vars: &WindowVars) {
        self.update_state(window_vars.state().get());
    }

    pub fn update(&mut self, window_vars: &WindowVars) {
        if let Some(state) = window_vars.state().get_new() {
            self.update_state(state);
        }
    }

    fn update_state(&mut self, state: WindowState) {
        self.restore_handle.set_enabled(state != WindowState::Normal);
        self.maximize_handle.set_enabled(state != WindowState::Maximized);
        self.minimize_handle.set_enabled(state != WindowState::Minimized);
    }
}
