# Focus Notes

## Requirements

* Focus navigation
  * Default:
    * Forward(Tab) and Backward(Shift+Tab)
    * Arrow keys(?)
  * Manual:
    * Focus on an element manually  
  * Support for disabling the various Default focus navigations(like a textarea or file navigation):
    * Default on some elements
    * Users can change it on specific elements
  * A configurable starting point for the focus navigation
    * Default starting point
  * Support for dynamically chosing if an element is focusable or not

* Support for Focus Scopes
  * Each scope can have a saved focus inside it
  * When focus is returned to the scope the focus inside it is restored
  * The saved focus inside the scope can look different even if the scope is not focused on

* Support special navigation areas
  * Areas that are only for TAB but not arrows

## API

* FocusKey
  * NextUpdate::focus(NavRequest)
  * enum NavRequest { Direct(key), Next, Prev, Escape(Pop?), Left, Right, Up, Down }
* NextFrame::focusable_area(rectangle, key, starting_point: bool)
  * Focus map generated with frame.
    * Focus area rectangles
    * Nested rectangles, where some are focus scopes?
* Window only calls focus_navigate(..) if the key press is not handled.
  * Window holds state.
* Ui::focus_state_changed(focus_state: &struct FocusState)
* FocusState::key_state(key) -> KeyState
  * enum KeyState { NotFocused, Focused(Active/NotActive) }
