* Finish smooth scrolling.
   - smooth scroll to for `home` and `end` commands
* Animation, see `Variables.md`.

* Fix/test viewport units.
    - Optional/default redefine viewport size to scrollable viewport size.
* Rename UiNodeList::widget_ to item_.

* Review layout double-pass of stacks.
* Fix text final size, either clip or return accurate size.
* Get Widget from UiNode (at least to avoid having to do custom reference frames in fill_node).