### Modifier key names
### 
### * The ID is the `ModifierGesture` variant name. [1]
### * An OS generic text must be provided, optional OS specific text can be set as attributes.
### * OS attribute is a `std::env::consts::OS` value. [2]
### 
### [1]: https://zng-ui.github.io/doc/zng/gesture/enum.ModifierGesture.html
### [2]: https://doc.rust-lang.org/std/env/consts/constant.OS.html

Alt = Alt
    .macos = ⌥Option

Ctrl = Ctrl
    .macos = ^Control

Shift = ⇧Shift

Super = Super
    .macos = ⌘Command
    .windows = ⊞Win
