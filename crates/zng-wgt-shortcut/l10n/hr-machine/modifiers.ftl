### Machine translated by `cargo zng l10n`, 80347e459a2de6a0783a295d0ef30d2a78a9f12622c7f68018251fd94b105330

### Automatski generirano pomoću `cargo zng l10n`

### Nazivi modifikatora tastera
### 
### * ID je naziv varijante `ModifierGesture`. [1]
### * Mora biti dostavljen opći tekst za OS, opcionalni OS-specifični tekst se može postaviti kao atribut.
### * OS atribut je vrijednost `std::env::consts::OS`. [2]
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