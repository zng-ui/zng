# Command TODO

Commands core API is implemented, TODO implement extensions and test all use cases.

* Make default `command_property!` be widget is enabled.
* Add filter to `command_property!`.
  - We want to declare `on_cmd_disabled` like for some event properties do.

* Localize provided commands.
  - Don't need to actually translate, just `l10n!` it.
* Implement *check-box* commands, probably an extension, need to test in menus when that is implemented.
* Implement `Command::focused_scope`.
  - For commands that work on the current focus or return-focus.
  - Like undo, or paste?
    - Yes, editors like VSCode Edit->Undo applies to the focused text area.
    - Needs to be a `Var<Command>`?
      - Can `flat_map(|c| c.is_enabled())`.
  - Not just focused, custom info query?
    - Scope may be a parent of the current focus.
  - Extension?
    - `CommandFocusExt::focused_scope` and `CommandFocusExt::focused_scope`.