# Focus TODO

* Improve directional nav by hitting more points, not just the center?

* Review focus parent/child.
   - Refactor directional to skip focusable content, only enter goes in?
   - Directional can escape?
   - Review focusable scopes.

* Restore focus from `modal` focus scope to button that opened it. 
* Restore focus to nearest sibling when focused is removed.
* Support more then one ALT scopes?
* Test directional navigation.
   * Including layout transformed widgets.
* Mnemonics.

## Icon Example

* Focus moving to scrollable when using left and right arrows in the middle row
* Focus moving twice when cycling from one of the icons
* The priority of keyboard focus should be high when highlighting and low when not?
* `move_focus` changes highlight to true when scrolling with arrow keys