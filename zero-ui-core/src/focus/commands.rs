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

    /// Represents the **focus alt** action.
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

    /// Represents the **escape alt** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Escape Alt"                                          |
    /// | [`info`]     | "Escape alt scope."                                   |
    /// | [`shortcut`] | `Escape`, `Alt`                                       |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub EscapeAltCommand
        .init_name("Escape Alt")
        .init_info("Escape alt scope.")
        .init_shortcut([shortcut!(Escape), shortcut!(Alt)]);

    /// Represents the **focus child** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Child"                                         |
    /// | [`info`]     | "Focus child focusable"                               |
    /// | [`shortcut`] | `Enter`, `ALT+Enter`                                  |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusChildCommand
        .init_name("Focus Child")
        .init_info("Focus Child focusable.")
        .init_shortcut([shortcut!(Enter), shortcut!(ALT+Enter)]);

    /// Represents the **focus parent** action.
    ///
    /// # Metadata
    ///
    /// This command initializes with the following metadata:
    ///
    /// | metadata     | value                                                 |
    /// |--------------|-------------------------------------------------------|
    /// | [`name`]     | "Focus Parent"                                        |
    /// | [`info`]     | "Focus parent focusable"                              |
    /// | [`shortcut`] | `Escape`, `ALT+Escape`                                |
    ///
    /// [`name`]: CommandNameExt
    /// [`info`]: CommandInfoExt
    /// [`shortcut`]: CommandShortcutExt
    pub FocusParentCommand
        .init_name("Focus Parent")
        .init_info("Focus parent focusable.")
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
    esc_handle: CommandHandle,

    up_handle: CommandHandle,
    down_handle: CommandHandle,
    left_handle: CommandHandle,
    right_handle: CommandHandle,

    parent_handle: CommandHandle,
    child_handle: CommandHandle,

    #[allow(dead_code)]
    focus_handle: CommandHandle,
}
impl FocusCommands {
    pub fn new(events: &mut Events) -> Self {
        Self {
            next_handle: FocusNextCommand.new_handle(events, false),
            prev_handle: FocusPrevCommand.new_handle(events, false),

            alt_handle: FocusAltCommand.new_handle(events, false),
            esc_handle: EscapeAltCommand.new_handle(events, false),

            up_handle: FocusUpCommand.new_handle(events, false),
            down_handle: FocusDownCommand.new_handle(events, false),
            left_handle: FocusLeftCommand.new_handle(events, false),
            right_handle: FocusRightCommand.new_handle(events, false),

            parent_handle: FocusParentCommand.new_handle(events, false),
            child_handle: FocusChildCommand.new_handle(events, false),

            focus_handle: FocusCommand.new_handle(events, true),
        }
    }

    pub fn update_enabled(&mut self, nav: FocusNavAction) {
        self.next_handle.set_enabled(nav.contains(FocusNavAction::NEXT));
        self.prev_handle.set_enabled(nav.contains(FocusNavAction::PREV));

        self.alt_handle.set_enabled(nav.contains(FocusNavAction::ALT));
        self.esc_handle.set_enabled(nav.contains(FocusNavAction::ESCAPE_ALT));

        self.up_handle.set_enabled(nav.contains(FocusNavAction::UP));
        self.down_handle.set_enabled(nav.contains(FocusNavAction::DOWN));
        self.left_handle.set_enabled(nav.contains(FocusNavAction::LEFT));
        self.right_handle.set_enabled(nav.contains(FocusNavAction::RIGHT));

        self.parent_handle.set_enabled(nav.contains(FocusNavAction::PARENT));
        self.child_handle.set_enabled(nav.contains(FocusNavAction::CHILD));
    }

    pub fn event_preview<EV: EventUpdateArgs>(&mut self, ctx: &mut AppContext, args: &EV) {
        macro_rules! handle {
            ($($Command:ident($handle:ident) => $method:ident,)+) => {$(
                if let Some(args) = $Command.update(args) {
                    args.handle_enabled(&self.$handle, |_| {
                        ctx.services.focus().$method();
                    });
                    return;
                }
            )+};
        }
        handle! {
            FocusNextCommand(next_handle) => focus_next,
            FocusPrevCommand(prev_handle) => focus_prev,
            FocusAltCommand(alt_handle) => focus_alt,
            EscapeAltCommand(esc_handle) => escape_alt,
            FocusUpCommand(up_handle) => focus_up,
            FocusDownCommand(down_handle) => focus_down,
            FocusLeftCommand(left_handle) => focus_left,
            FocusRightCommand(right_handle) => focus_right,
            FocusChildCommand(child_handle) => focus_child,
            FocusParentCommand(parent_handle) => focus_parent,
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
