# Fix context var map

* Review if all mapping variables re-compute inside the same update cycle, some variables may check the update ID?
    - CowVar Ok.
    - FilterMap And BidiFilterMap Fixed.
    - FlatMap Ok.
    - MapRef and BidiMapRef Ok.
    - Map and BidiMap Ok.
    - merge_var! Fixed.
    - switch! Ok.
    - RcSwitchVar Ok.
    - when_var! Ok.
    - RcWhenVar ?

# Finish scroll

* Implement ContextVar modify.