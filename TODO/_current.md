# TextInput

* Implement selection.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement automation/screen reader APIs.

# Touch Events

* Implement `TOUCH` service and events.
    - Track fingers in vars only.
    - And simple gestures like tap.
* Integrate with `GESTURES`.
    - Tap with click.
    - Complex gestures (pinch, etc.) here too?
        - Because of the name "gestures"?

```log
// Log of tab and a drag.

# Touch (phase, pos, force, finger_id)

Touch (Started, (425dip, 310dip), Some(Normalized(0.5)), 25)
Touch (Moved, (425dip, 310dip), Some(Normalized(0.5)), 25)
Touch (Ended, (425dip, 310dip), Some(Normalized(0.5)), 25)

Touch (Started, (458dip, 379dip), Some(Normalized(0.5)), 26)
Touch (Moved, (458dip, 379dip), Some(Normalized(0.5)), 26)
Touch (Moved, (458dip, 379dip), Some(Normalized(0.5)), 26)
Touch (Moved, (453dip, 361dip), Some(Normalized(0.5)), 26)
Touch (Moved, (451dip, 354dip), Some(Normalized(0.5)), 26)
Touch (Moved, (448dip, 347dip), Some(Normalized(0.5)), 26)
Touch (Moved, (445dip, 342dip), Some(Normalized(0.5)), 26)
Touch (Moved, (423dip, 243dip), Some(Normalized(0.5)), 26)
Touch (Moved, (423dip, 243dip), Some(Normalized(0.5)), 26)
Touch (Moved, (423dip, 243dip), Some(Normalized(0.5)), 26)
Touch (Moved, (423dip, 243dip), Some(Normalized(0.5)), 26)
Touch (Moved, (425dip, 247dip), Some(Normalized(0.5)), 26)
Touch (Ended, (425dip, 247dip), Some(Normalized(0.5)), 26)
```
