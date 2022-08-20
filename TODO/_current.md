# Bug

* Fix pre-build view-process is always software mode because it fails to create `Dedicated`.
```txt
Render mode selection log:

[Integrated]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
[Dedicated]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")
[Dedicated (generic)]
OsError("GetClassInfoExW function failed: Esta classe não existe. (os error 1411)")

```
1411 is ERROR_CLASS_DOES_NOT_EXIST

* Software mode rendering in the `image` example is much slower in the `theme-use` branch.
    - Bug is not caused by button theme, using the old button in the new branch was still slow.
    - Bug is not caused by any change in image example code, old image code was still slow.

# Text

* Text Editable
    - Caret.
    - Selection.
* `text_input!`.
    - Inherit from `text!`.
    - Appearance of a text-box.
* IME.
* `LineBreakVar`.
    - When char is `\n` or `\r` read this var and insert it instead. 
    - Review https://en.wikipedia.org/wiki/Newline
