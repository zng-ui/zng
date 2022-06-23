//! Commands that control focus.

use crate::{command::*, gesture::*};

use super::*;

command! {
    /// Represents the **focus next** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Next"                                          |
    /// | [`info`]     | "Focus next focusable."                               |
    /// | [`shortcut`] | `Tab`                                                 |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusNextCommand
        .init_name("Focus Next")
        .init_info("Focus next focusable.")
        .init_shortcut([shortcut!(Tab)]);

    /// Represents the **focus previous** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Previous"                                      |
    /// | [`info`]     | "Focus previous focusable."                           |
    /// | [`shortcut`] | `SHIFT+Tab`                                           |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusPrevCommand
        .init_name("Focus Previous")
        .init_info("Focus previous focusable.")
        .init_shortcut([shortcut!(SHIFT+Tab)]);

    /// Represents the **focus/escape alt** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Alt"                                           |
    /// | [`info`]     | "Focus alt scope."                                    |
    /// | [`shortcut`] | `Alt`                                                 |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusAltCommand
        .init_name("Focus Alt")
        .init_info("Focus alt scope.")
        .init_shortcut([shortcut!(Alt)]);

    /// Represents the **focus enter** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Enter"                                         |
    /// | [`info`]     | "Focus child focusable."                              |
    /// | [`shortcut`] | `Enter`, `ALT+Enter`                                  |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusEnterCommand
        .init_name("Focus Enter")
        .init_info("Focus child focusable.")
        .init_shortcut([shortcut!(Enter), shortcut!(ALT+Enter)]);

    /// Represents the **focus exit** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Exit"                                          |
    /// | [`info`]     | "Focus parent focusable, or return focus."            |
    /// | [`shortcut`] | `Escape`, `ALT+Escape`                                |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusExitCommand
        .init_name("Focus Exit")
        .init_info("Focus parent focusable, or return focus.")
        .init_shortcut([shortcut!(Escape), shortcut!(ALT+Escape)]);

    /// Represents the **focus up** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Up"                                            |
    /// | [`info`]     | "Focus closest focusable up."                         |
    /// | [`shortcut`] | `Up`, `ALT+Up`                                        |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusUpCommand
        .init_name("Focus Up")
        .init_info("Focus closest focusable up.")
        .init_shortcut([shortcut!(Up), shortcut!(ALT+Up)]);

    /// Represents the **focus down** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Down"                                          |
    /// | [`info`]     | "Focus closest focusable down."                       |
    /// | [`shortcut`] | `Down`, `ALT+Down`                                    |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusDownCommand
        .init_name("Focus Down")
        .init_info("Focus closest focusable down.")
        .init_shortcut([shortcut!(Down), shortcut!(ALT+Down)]);

    /// Represents the **focus left** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Left"                                          |
    /// | [`info`]     | "Focus closest focusable left."                       |
    /// | [`shortcut`] | `Left`, `ALT+Left`                                    |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusLeftCommand
        .init_name("Focus Left")
        .init_info("Focus closest focusable left.")
        .init_shortcut([shortcut!(Left), shortcut!(ALT+Left)]);

    /// Represents the **focus right** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Right"                                         |
    /// | [`info`]     | "Focus closest focusable right."                      |
    /// | [`shortcut`] | `Right`, `ALT+Right`                                  |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusRightCommand
        .init_name("Focus Right")
        .init_info("Focus closest focusable right.")
        .init_shortcut([shortcut!(Right), shortcut!(ALT+Right)]);

    /// Represents a [`FocusRequest`] action.
    ///
    /// If this command parameter is a [`FocusRequest`] the request is made.
    pub FocusCommand;
}

pub(super) struct FocusCommands {
    next_handle: CommandHandle,
    prev_handle: CommandHandle,

    alt_handle: CommandHandle,

    up_handle: CommandHandle,
    down_handle: CommandHandle,
    left_handle: CommandHandle,
    right_handle: CommandHandle,

    exit_handle: CommandHandle,
    enter_handle: CommandHandle,

    #[allow(dead_code)]
    focus_handle: CommandHandle,
}
impl FocusCommands {
    pub fn new(events: &mut Events) -> Self {
        Self {
            next_handle: FocusNextCommand.new_handle(events, false),
            prev_handle: FocusPrevCommand.new_handle(events, false),

            alt_handle: FocusAltCommand.new_handle(events, false),

            up_handle: FocusUpCommand.new_handle(events, false),
            down_handle: FocusDownCommand.new_handle(events, false),
            left_handle: FocusLeftCommand.new_handle(events, false),
            right_handle: FocusRightCommand.new_handle(events, false),

            exit_handle: FocusExitCommand.new_handle(events, false),
            enter_handle: FocusEnterCommand.new_handle(events, false),

            focus_handle: FocusCommand.new_handle(events, true),
        }
    }

    pub fn update_enabled(&mut self, nav: FocusNavAction) {
        self.next_handle.set_enabled(nav.contains(FocusNavAction::NEXT));
        self.prev_handle.set_enabled(nav.contains(FocusNavAction::PREV));

        self.alt_handle.set_enabled(nav.contains(FocusNavAction::ALT));

        self.up_handle.set_enabled(nav.contains(FocusNavAction::UP));
        self.down_handle.set_enabled(nav.contains(FocusNavAction::DOWN));
        self.left_handle.set_enabled(nav.contains(FocusNavAction::LEFT));
        self.right_handle.set_enabled(nav.contains(FocusNavAction::RIGHT));

        self.exit_handle.set_enabled(nav.contains(FocusNavAction::EXIT));
        self.enter_handle.set_enabled(nav.contains(FocusNavAction::ENTER));
    }

    pub fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        macro_rules! handle {
            ($($Command:ident($handle:ident) => $method:ident,)+) => {$(
                if let Some(args) = $Command.update(args) {
                    args.handle(|args| {
                        if args.enabled && self.$handle.is_enabled() {
                            ctx.services.focus().$method();
                        } else {
                            ctx.services.focus().on_disabled_cmd();
                        }
                    });
                    return;
                }
            )+};
        }
        handle! {
            FocusNextCommand(next_handle) => focus_next,
            FocusPrevCommand(prev_handle) => focus_prev,
            FocusAltCommand(alt_handle) => focus_alt,
            FocusUpCommand(up_handle) => focus_up,
            FocusDownCommand(down_handle) => focus_down,
            FocusLeftCommand(left_handle) => focus_left,
            FocusRightCommand(right_handle) => focus_right,
            FocusEnterCommand(enter_handle) => focus_enter,
            FocusExitCommand(exit_handle) => focus_exit,
        }

        if let Some(args) = FocusCommand.update(args) {
            if let Some(req) = args.param::<FocusRequest>() {
                args.handle_enabled(&self.focus_handle, |_| {
                    ctx.services.focus().focus(*req);
                });
            }
        }
    }
}
