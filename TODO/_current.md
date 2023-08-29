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

* Resources
    - https://developer.mozilla.org/en-US/docs/Web/API/TouchEvent
    - https://searchfox.org/mozilla-central/source/gfx/layers/apz/src/GestureEventListener.cpp#233
    - https://api.flutter.dev/flutter/gestures/gestures-library.html

* `gesture_propagation`:
    - Both extensions and properties can implement this.
    - Create property helper, only subscribes to touch move when gesture pending?
    - Loosen-up touch tap area.

* Integrate with `GESTURES`.
    - Basic gestures:
        - Pinch:
            - Two fingers, move closer together or a farther apart.
        - Rotate:
            - Two finger, move around each other.
        - Pan or drag:
            - One finger, press, move.
        - All 3 can happen at the same time.
    - Inertia:
        - A pan/drag can be "thrown".
            - One finger, press, move, release "while moving".
            - Pan continues for a time.
            - User controls velocity to some extent.
    - Tension:
        - Gestures can "push" against a constraint.
        - Pan scroll has visual feedback when it can't scroll a direction anymore.
            - Different from no feedback when you can never scroll in a direction.
    - Long press (from mouse too?).

* Add event properties.
* Review/add state properties.
    - Review `is_pointer_pressed`.
        - Is only for mouse with primary button currently.
        - Add touch, review what is a pressed touch.
            - Need to show pressed only if a tap will happen on touch-end?
                - Canceled by multiple causes.
                - May need a "promise" var for the future tap, `ResonseVar<bool>`.
    - Add `is_mouse_pressed` and `is_touch_pressed`.

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
