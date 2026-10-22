### Machine translated by `cargo zng l10n`, 80347e459a2de6a0783a295d0ef30d2a78a9f12622c7f68018251fd94b105330

### Generat automàticament per `cargo zng l10n`

### Noms de les tecles modificadores
### 
### * L'ID és el nom de la variant `ModifierGesture`. [1]
### * S'ha de proporcionar un text genèric del sistema operatiu; es pot configurar text específic del sistema operatiu com a atribut.s.
### * L'atribut del sistema operatiu és un valor `std::env::consts::OS`. [2]
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