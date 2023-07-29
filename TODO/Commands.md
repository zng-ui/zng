# Command TODO

Commands core API is implemented, TODO implement extensions and test all use cases.

* Make default `command_property!` be widget is enabled.
* Add filter to `command_property!`.
  - We want to declare `on_cmd_disabled` like for some event properties do.
* Make `command_property!` generate an user controlled enabled?
  - Some commands depend on the content, can only `SAVE_AS` if there is a document open for example.
  - Right now the command is always enabled.

* Localize provided commands.
  - Don't need to actually translate, just `l10n!` it.
  - Very verbose?
    - Maybe we can implement an exception in the scrapper?
      - Like a `.l10n("cmd-key").name("Fallback Name")`.
      - Needs to be supported in command extensions too.
        - Maybe `.l10n` returns a different type?
  - Localize shortcuts?
    - Some apps localize some shortcuts.
    - Implement `l10n_parse!`?

* Implement automatic "alt" shortcuts.
  - When you press alt the command buttons inside the alt scope change the
    text to highlight the first unique char that can be pressed to receive
    a click from the command.
