### Machine translated by `cargo zng l10n`, 80347e459a2de6a0783a295d0ef30d2a78a9f12622c7f68018251fd94b105330

### Geautomatiseerd door `cargo zng l10n`

### Namen van modifier-toetsen
### 
### * De ID is de variantnaam `ModifierGesture`. [1]
### * Er moet een algemene OS-tekst worden verstrekt; optionele OS-specifieke tekst kan als attributen worden ingesteld.
### * Een OS-attribuut is een `std::env::consts::OS` waarde. [2]
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