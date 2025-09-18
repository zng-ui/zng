### Valid gesture key names
### 
### * The ID is the `Key` variant name. [1]
### * An OS generic text must be provided, optional OS specific text can be set as attributes.
### * OS attribute is a `std::env::consts::OS` value. [2]
### * L10n not include Char, Str, modifiers and composite keys.
### 
### Note: This file does not include all valid keys, see [1] for a full list.
### 
### [1]: https://zng-ui.github.io/doc/zng/keyboard/enum.Key.html
### [2]: https://doc.rust-lang.org/std/env/consts/constant.OS.html

ArrowDown = ↓

ArrowLeft = ←

ArrowRight = →

ArrowUp = ↑

Backspace = ←Backspace
    .macos = Delete

Close = Close

ContextMenu = ≣Context Menu

Copy = Copy

Cut = Cut

Delete = Delete
    .macos = Forward Delete

Eject = ⏏Eject

Enter = ↵Enter
    .macos = ↵Return

Escape = Esc

Find = Find

Help = ?Help

New = New

Open = Print

PageDown = PgDn

PageUp = PgUp

Paste = Paste

PrintScreen = PrtSc

Redo = Redo

Save = Save

Tab = ⭾Tab

Undo = Undo

ZoomIn = +Zoom In

ZoomOut = -Zoom Out
