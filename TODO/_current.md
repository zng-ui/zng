# TextInput

* Implement selection.
    - Mouse:
        - Press and drag selection.
        - Double-click causes selection that grows by word.
        - Triple-click and Quadruple-click?
    - Keyboard:
        - Holding shift and pressing the arrow keys.
    - Touch:
        - Research how it works.
    - Input replaces selection.
        - Char input, paste, IME
    - Impl cut & copy.

* Implement IME.
    - See https://github.com/rust-windowing/winit/issues/1497

# View-Process

* Update to winit-29 when released.
    - Lots of breaking changes.
* Implement automation/screen reader APIs.

# Events

* Mouse and touch event properties do not check `capture_allows`?

# Scroll

* Implement over-scroll indicator.
    - Animation release.
* Implement touch scroll inertia.
* Implement `ScrollMode::ZOOM`.
    - Touch gesture.
    - Scroll wheel zoom.
    - Commands.

# Touch Events

* Implement `TapMode`?
    - Like `ClickMode`.
    - We want the same gesture in combo box of clicking and dragging to the option.
    - Maybe use the swipe/fling gesture?

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
    - Pinning:
        - One finger rotates around a fixed point.
    - Force press.
        - Normalized force is 0.5 (for touchscreens without force detection).
        - This gesture exceeds this force.

* Implement "Ripple!" visual.
    - Radial background overlay animation fired on every touch or click.

* Improve `touch_transform`.
    - Contextual origin.
    - Config persistence.

# Exclusive Video Mode

* How is the scale factor selected for exclusive?
* Settings to a smaller size (800x600) causes background clipping.
* Also test in non-primary monitor.