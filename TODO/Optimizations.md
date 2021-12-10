# Build Time

* Very slow build time in release mode without `dyn_widget` (window example up-to 18 minutes compile time and 25GB memory usage).
    Might be related to https://github.com/rust-lang/rust/issues/75992

# Mouse Move Interest

* Let widgets define what sort of mouse event they want, use the hit-test tag, filter events in the view-process.

# Update Mask

* Sub-divide UiNodeList masks.

# Events

* Replace `EventArgs::concerns_widget` with a `EventsArgs::target` that is an `Option<&WidgetPath>`,
  widgets can then route the event more efficiently, specially for cases like the cursor move where
  most of the widgets are subscribing to the event type but only a small portion of then are going to receive.

  Target `None` are only for `AppExtensions`, and maybe the window root?.

# Startup

* NVIDIA OpenGL takes 200ms! to startup.
* First render is also slow.
* We block the app process waiting view-process startup.

# Cache Everything

General idea, reuse computed data for `UiNode` info, layout and render at
widget boundaries if the widget or inner widgets did not request an update of these types.

## `UiNode::subscriptions` 

Easiest to do, can serve as a test for the others?

## `UiNode::info`

Could probably look the same as `subscriptions` but can an ego-tree be build from sub-trees?

To cache metadata we need to clone-it, `AnyMap` is not cloneable, could `Rc` the map.

## `UiNode::render`

Webrender needs to support this, check how they do `<iframe>`?

Has potential to use add megabytes of memory use, lots of repeating nested content, 
maybe we dynamically change what widget must cache based on use.

## Layout

Most difficult, can depend on context available size, font size, view-port size, can it be done?