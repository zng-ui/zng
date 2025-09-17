# Modifier key names
#
# * The ID is the `ModifierGesture` variant name.
# * A OS generic text must be provided, optional OS specific text can be set as attributes.
# * OS attribute is a `std::env::consts::OS` value.
#
# Note: macOS does not localize modifier names

Super = Super
    .macos = ⌘Command
    .windows = ⊞Win

Ctrl = Ctrl
    .macos = ^Control

Shift = ⇧Shift

Alt = Alt
    .macos = ⌥Option
