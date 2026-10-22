### Machine translated by `cargo zng l10n`, 80347e459a2de6a0783a295d0ef30d2a78a9f12622c7f68018251fd94b105330

### Automatizált generálás a `cargo zng l10n` által

### Módosító kulcsnevek
### 
### * Az ID a `ModifierGesture` variáns neve. [1]
### * Egy OS általános szöveg meg kell adni, opcionális OS specifikus szöveget attribútumként lehet beállítani.
### * Az OS attribútum egy `std::env::consts::OS` érték. [2]
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