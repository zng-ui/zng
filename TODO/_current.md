# TextInput

* Implement selection.
    - Use `CaretInfo::selection_range`.
    - Input replaces selection.
        - Char input, paste, IME
    - Double-click causes selection that grows by word.
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement automation/screen reader APIs.

# Touch Events

* Implement `TOUCH_TRANSFORM_EVENT`.
    - Need to wait for the tap distance to clear to start the event?
        - No, because its two fingers.
* Implement `TOUCH_PAN_EVENT`.
    - One finger translate.
    - Needs to wait for the tap distance.
    - Can we integrate this in the transform event?

* Implement `TapMode`?
    - Like `ClickMode`.
    - We want the same gesture in combo box of clicking and dragging to the option.
    - Maybe actually have a drag down gesture expand the combo?

* Gesture propagation:
    - Create property helper, only subscribes to touch move when gesture pending?
    - OR, a `GESTURES.register_gesture` or with a boxed trait.
    - OR, events that check `has_hooks` and `is_subscriber`.
    - Setting a `on_touch_gesture` only in `when` still subscribes to the event always.
        - Because we move the when resolver inside an event handler.
        - Refactor `WidgetHandler` to signal when it is nil?
    - CLICK_EVENT is just an aggregator that includes TOUCH_TAP_EVENT, but the gestures manager
      must subscribe to it globally to work.
        - Ideally we only include the subscribers of CLICK_EVENT in TOUCH_TAP_EVENT.

* Implement gestures:
    - Context tap.
        - Long press?
        - CLICK_EVENT has interest in these, but if we always generate then it will conflict with pan?
    - Pinch:
        - Two fingers, move closer together or a farther apart.
    - Rotate:
        - Two finger, move around each other.
    - Pan or drag:
        - One finger, press, move.
    - All 3 can happen at the same time.
        - Same event?
        - TOUCH_TRANSFORM_EVENT.
            - Tracks two points and computes a transform on demand for it.
            - Transform can optionally include
    - Inertia:
        - A pan/drag can be "thrown".
            - One finger, press, move, release "while moving".
            - Pan continues for a time.
            - User controls velocity to some extent.
    - Tension:
        - Gestures can "push" against a constraint.
        - Pan scroll has visual feedback when it can't scroll a direction anymore.
            - Different from no feedback when you can never scroll in a direction.
    - Pinning:
        - One finger rotates around a fixed point.
    - Long press (from mouse too?).
    - Force press.
        - Normalized force is 0.5 (for touchscreens without force detection).
        - This gesture exceeds this force.

* Implement "Ripple!" visual.
    - Radial background overlay animation fired on every touch or click.

* Touch drag release in the window background causes the title bar to not be interactive anymore.
    - Sometimes can't recreate, but it has happened multiple times already.
    - Maybe only when window is between screens only?