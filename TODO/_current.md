# Light Theme

* Review light theme in all examples.

- example    - description
- border     - text color
- calculator - display number color is not themed
- config     - text input and update status are not themed
- countdown  - text color
- cursor     - theme not implemented in cursor areas
- focus      - command status, focus target and overlay scope are not themed
 - - nested  - theme also needs tweaking
- icon       - icons and icon cards are not themed
- image      - section title backgrounds, the checkerboard background and the `loading...` placeholder are not themed
- layer      - Anchored and Layer (7,8,9) button overlays are not themed, the TOP_MOST overlay is not themed either
- respawn    - status background is not themed
- scroll     - background color and commands menu background color are not themed
- shortcuts  - shortcut text color is not themed

- text       - colored text is hard to see in light theme
             - font size widget background is not themed

- transform  - red

# Dyn Widget 2

* Refactor dynamic widget to have the normal constructor functions, except `new` that becomes `new_dyn`.

```rust
fn new_context(child: impl UiNode, capture: impl IntoVar<bool>) -> impl UiNode {
    // normal constructor, no `new_context_dyn`, but if the widget has a `new_dyn` the `child` is an
    // AdoptiveNode placeholder.
    child
}

fn new_dyn(widget: DynWidget, id: impl IntoVar<WidgetId>) -> CustomType {
    // only constructor that has a `_dyn` alt, if `_dyn` the required paramenter is not a `impl UiNode`, it is the dynamic
    // widget. 
}

struct DynWidget {
    parts: [DynWidgetPart, _],
    whens: Vec<DynWhen>
}

struct DynWidgetPart {
    // `context` properties, sorted like before.
    properties: Vec<DynProperties>,
    // node returned by `new_context`.
    intrinsic: AdoptiveNode<BoxedUiNode>,
}
```

## Why?

* Required to implement dynamic `when` support, we can't separate when blocks are a single unit, but affect multiple priorities.
* It is more easy to inherit from an existing static widget and turns it dynamic, just need to patch the `new_dyn`, right now
  we had to implement a `text_mixin` to implement `text_input`, after this change we can just inherit from `text`.