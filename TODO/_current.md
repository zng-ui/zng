# TextInput

* Implement selection.
    - Implement `txt_highlight` first.
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
    - Copy `MOUSE` patterns.
* `TOUCH_DOWN/MOVE/UP`.
* `TOUCH_ENTER/LEAVE`.
* `TAP_EVENT`.            
* Touch capture like mouse capture?
    - Required, at least WPF does this.
    - Can use the same types (only docs change).
    - Can unify both in a `CAPTURE` service?
        - Is deeply integrated with these events.
            - Uses the same raw events.
        - Can only capture if mouse is down or touch is down.
        - Can capture on press (this is the integrated part).

* Integrate with `GESTURES`.
    - `CLICK_EVENT` from `TAP_EVENT`.
    - Basic gestures:
        - Pinch:
            - Two fingers, move closer together or a farther apart.
        - Rotate:
            - Two finger, move around each other.
        - Pan or drag:
            - One finger, press, move.
        - All 3 can happen at the same time.
            - Same event?
        - Gestures can be ambiguous?
           - `TOUCH_DOWN` in a button widget can be the start of a `TAP_EVENT` or of a pan gesture.
           - If the button moves with the pan gesture the `TOUCH_UP` still completes the tap?
           - Test what browsers do in this case.
    - Inertia:
        - A pan/drag can be "thrown".
            - One finger, press, move, release "while moving".
            - Pan continues for a time.
            - User controls velocity to some extent.
    - Tension:
        - Gestures can "push" against a constraint.
        - Pan scroll has visual feedback when it can't scroll a direction anymore.
            - Different from no feedback when you can never scroll in a direction.
     

    - `LONG_PRESS_EVENT` (from mouse too?).

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

// Can also have a Canceled phase.
```
