//! Commands that control the scoped window.
//!
//! The window widget implements all these commands scoped to the window ID.

use zero_ui::core::{
    command::*,
    context::{InfoContext, WidgetContext},
    event::EventUpdateArgs,
    gesture::*,
    var::*,
    widget_info::WidgetSubscriptions,
    window::{WindowVarsKey, WindowsExt},
    *,
};
use zero_ui_core::window::WindowState;

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
    /// This command is about the *exclusive* fullscreen state ([`WindowSTate::Exclusive`]),
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

pub(super) fn window_control_node(child: impl UiNode) -> impl UiNode {
    struct WindowControlNode<C> {
        child: C,

        maximize_handle: CommandHandle,
        minimize_handle: CommandHandle,
        restore_handle: CommandHandle,

        fullscreen_handle: CommandHandle,
        exclusive_handle: CommandHandle,

        close_handle: CommandHandle,

        state_var: Option<RcVar<WindowState>>,

        allow_alt_f4_binding: Option<VarBindingHandle>,
    }
    impl<C> WindowControlNode<C> {
        fn update_state(&mut self, state: WindowState) {
            self.restore_handle.set_enabled(state != WindowState::Normal);
            self.maximize_handle.set_enabled(state != WindowState::Maximized);
            self.minimize_handle.set_enabled(state != WindowState::Minimized);
        }
    }
    #[impl_ui_node(child)]
    impl<C: UiNode> UiNode for WindowControlNode<C> {
        fn subscriptions(&self, ctx: &mut InfoContext, subs: &mut WidgetSubscriptions) {
            let scope = ctx.path.window_id();

            subs.event(MaximizeCommand.scoped(scope))
                .event(MinimizeCommand.scoped(scope))
                .event(FullscreenCommand.scoped(scope))
                .event(ExclusiveFullscreenCommand.scoped(scope))
                .event(RestoreCommand.scoped(scope))
                .event(CloseCommand.scoped(scope))
                .var(ctx, self.state_var.as_ref().unwrap());

            self.child.subscriptions(ctx, subs);
        }

        fn init(&mut self, ctx: &mut WidgetContext) {
            let window_id = ctx.path.window_id();

            // state
            self.maximize_handle = MaximizeCommand.scoped(window_id).new_handle(ctx, false);
            self.minimize_handle = MinimizeCommand.scoped(window_id).new_handle(ctx, false);
            self.fullscreen_handle = FullscreenCommand.scoped(window_id).new_handle(ctx, true);
            self.exclusive_handle = ExclusiveFullscreenCommand.scoped(window_id).new_handle(ctx, true);
            self.restore_handle = RestoreCommand.scoped(window_id).new_handle(ctx, false);
            let state_var = ctx.window_state.req(WindowVarsKey).state().clone();
            self.update_state(state_var.copy(ctx));
            self.state_var = Some(state_var);

            // close
            self.close_handle = CloseCommand.scoped(window_id).new_handle(ctx, true);

            if cfg!(windows) {
                // hijacks allow_alt_f4 for the close command, if we don't do this
                // the view-process can block the key press and send a close event
                // without the CloseCommand event ever firing.
                let allow_alt_f4 = ctx.services.windows().vars(window_id).unwrap().allow_alt_f4();
                self.allow_alt_f4_binding = Some(
                    CloseCommand
                        .scoped(window_id)
                        .shortcut()
                        .bind_map(ctx.vars, allow_alt_f4, |_, s| s.contains(shortcut![ALT + F4])),
                );
            }

            self.child.init(ctx);
        }

        fn deinit(&mut self, ctx: &mut WidgetContext) {
            self.maximize_handle = CommandHandle::dummy();
            self.minimize_handle = CommandHandle::dummy();
            self.restore_handle = CommandHandle::dummy();

            self.fullscreen_handle = CommandHandle::dummy();
            self.exclusive_handle = CommandHandle::dummy();

            self.close_handle = CommandHandle::dummy();
            self.state_var = None;

            self.allow_alt_f4_binding = None;
            self.child.deinit(ctx);
        }

        fn event<A: EventUpdateArgs>(&mut self, ctx: &mut WidgetContext, args: &A) {
            let scope = ctx.path.window_id();
            let state_var = self.state_var.as_ref().unwrap();
            let restore_state = || ctx.window_state.req(WindowVarsKey).restore_state().copy(ctx.vars);

            if let Some(args) = MaximizeCommand.scoped(scope).update(args) {
                if self.maximize_handle.is_enabled() {
                    state_var.set_ne(ctx, WindowState::Maximized);
                }

                self.child.event(ctx, args);
                return;
            }

            if let Some(args) = MinimizeCommand.scoped(scope).update(args) {
                if self.minimize_handle.is_enabled() {
                    state_var.set_ne(ctx, WindowState::Minimized);
                }

                self.child.event(ctx, args);
                return;
            }

            if let Some(args) = RestoreCommand.scoped(scope).update(args) {
                if self.restore_handle.is_enabled() {
                    state_var.set_ne(ctx, restore_state());
                }

                self.child.event(ctx, args);
                return;
            }

            if let Some(args) = CloseCommand.scoped(scope).update(args) {
                if self.close_handle.is_enabled() {
                    let _ = ctx.services.windows().close(scope);
                }

                self.child.event(ctx, args);
                return;
            }

            if let Some(args) = FullscreenCommand.scoped(scope).update(args) {
                if self.fullscreen_handle.is_enabled() {
                    if let WindowState::Fullscreen = state_var.copy(ctx) {
                        state_var.set(ctx, restore_state());
                    } else {
                        state_var.set(ctx, WindowState::Fullscreen);
                    }
                }

                self.child.event(ctx, args);
                return;
            }

            if let Some(args) = ExclusiveFullscreenCommand.scoped(scope).update(args) {
                if self.exclusive_handle.is_enabled() {
                    if let WindowState::Exclusive = state_var.copy(ctx) {
                        state_var.set(ctx, restore_state());
                    } else {
                        state_var.set(ctx, WindowState::Exclusive);
                    }
                }

                self.child.event(ctx, args);
                return;
            }

            self.child.event(ctx, args);
        }

        fn update(&mut self, ctx: &mut WidgetContext) {
            if let Some(state) = self.state_var.as_ref().unwrap().copy_new(ctx) {
                self.update_state(state);
            }

            self.child.update(ctx);
        }
    }
    WindowControlNode {
        child: child.cfg_boxed(),

        maximize_handle: CommandHandle::dummy(),
        minimize_handle: CommandHandle::dummy(),
        restore_handle: CommandHandle::dummy(),

        fullscreen_handle: CommandHandle::dummy(),
        exclusive_handle: CommandHandle::dummy(),

        close_handle: CommandHandle::dummy(),

        state_var: None,

        allow_alt_f4_binding: None,
    }
    .cfg_boxed()
}

#[cfg(inspector)]
pub(super) fn inspect_node(child: impl UiNode, can_inspect: impl var::IntoVar<bool>) -> impl UiNode {
    use crate::core::inspector::{write_tree, WriteTreeState};

    let mut state = WriteTreeState::none();

    let can_inspect = can_inspect.into_var();

    on_command(
        child,
        |ctx| InspectCommand.scoped(ctx.path.window_id()),
        move |_| can_inspect.clone(),
        hn!(|ctx, args: &CommandArgs| {
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
