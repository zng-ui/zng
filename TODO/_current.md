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

* Implement `TapMode`?
    - Like `ClickMode`.
    - We want the same gesture in combo box of clicking and dragging to the option.
    - Maybe actually have a drag down gesture expand the combo?

* `gesture_propagation`:
    - Create property helper, only subscribes to touch move when gesture pending?
    - OR, a `GESTURES.register_gesture` or with a boxed trait.
    - We must only start trying a gesture if the target widget path subscribes to it.
        - Can use `Event::is_subscriber` for this.
        - And `Event::has_hooks`.
    - Refactor `TOUCH_TAP_EVENT` to use this.

* Implement gestures:
    - Double tap.
    - Context tap.
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
    - Force press.
        - Normalized force is 0.5 (for touchscreens without force detection).
        - This gesture exceeds this force.

* Implement "Ripple!" visual.
    - Radial background overlay animation fired on every touch or click.

* Touch drag release in the window background causes the title bar to not be interactive anymore.
    - Sometimes can't recreate, but it has happened multiple times already.
    - Maybe only when window is between screens only?