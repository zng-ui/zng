### Machine translated by `cargo zng l10n`, 80347e459a2de6a0783a295d0ef30d2a78a9f12622c7f68018251fd94b105330

### تم الإنشاء تلقائيًا بواسطة `cargo zng l10n`

### أسماء مفاتيح التعديل
### 
### * المعرّف (ID) هو اسم متغير `ModifierGesture`. [1]
### * يجب توفير نص عام لنظام التشغيل، مع إمكانية تعيين نص خاص بنظام تشغيل معين كسمات (attributes).
### * سمة نظام التشغيل هي قيمة من `std::env::consts::OS`. [2]
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