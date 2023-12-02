# TextInput

* Touch selection.
    - Test with RTL and bidirectional text.
    - Implement `touch_carets` touch drag.
        - Implement in the layered shape?
        - Hit-test area full shape rectangle.

* Implement selection toolbar.
    - Panics if there is a popup::close_delay and you hover one when there is more than one open at a time.
    - Like MS Word "Mini Toolbar" on selection and the text selection toolbar on mobile?
    - Implement `selection_toolbar_fn`.
        - Needs to open when a selection finishes creating (mouse/touch release)
            - And close with any interaction that closes POPUP + any mouse/touch/keyboard interaction with the Text widget.
```rust
TextInput! {
    txt = var_from("select text to show toolbar");
    text::selection_toolbar = Wgt! {
        size = 40;
        background_color = colors::GREEN.with_alpha(50.pct());
    }
}
```

# Accessibility

*  panicked at 'assertion failed: self.nodes.contains_key(&self.focus)'
    - Run icon example, search z, click a button.
* All examples must be fully useable with a screen reader.
    - Test OS defaults and NVDA.

# Split Core

* Color.
    - Filter needs layout::Length.
    - Needs impl_from_and_into_var.
    - Into view_api::RenderColor.
        - Can be decoupled.

* App API.
    - Needs UpdateDeliveryList, that needs WidgetInfo.
        - WidgetInfo only to iter over path (without alloc).
        - Can be decoupled by taking a trusted Iterator and WindowId.
    - Needs AnyEventArgs.
        - Must include events?
        - Could be just this trait.
    - Implementers of AppExtension will very likely import all the other stuff anyway.
    - App API is core without App::default?
        - Can we make it more similar to view-api?
        - It must include AppExtension and all it pulls, but not include view-api.
            - But must include raw events that view-api is mapped too.
            - Extensions need view-api as much as they need raw events.
* App.
    - Implements view controller and raw events.
    - Provides App, HeadlessApp.
    - Does not provide App::default()?
        - Could be on a feature flag.

* Config.
    - Needs app API.
    - Needs var.

* L10n.
    - Needs app API.
    - Needs var.

* Text Shaping.
    - Mostly decoupled, needs l10n's Lang.

* Focus.
    - Needs app API.
    - Needs WINDOWS.

- File-System Watcher
    - Needs app API.
    - Needs timers.

- Gesture
    - Needs app API.
    - Needs WINDOWS, keyboard and mouse.

- Image
    - Needs app API.
    - Needs ViewImage.

- Keyboard, mouse and touch.
    - Needs app API.
    - Needs FOCUS.
    - Needs TIMERS.

- Pointer Capture.
    - Needs mouse and touch.
    - Needs WINDOWS.

- TIMERS.
    - Could decouple like VARS maybe.
    - UPDATES controls it, and is called by it.

- Undo.
    - Needs app API.

- WINDOWS.
    - Needs app API.
    - Needs ViewWindow.
    - Needs UiNode.

- UiNode, widget_base, widget_info, widget_builder.
    - Needs app API for update types?
    - Needs to implement Var::subscribe.


## Split Main

* Per widget?

# Publish

* Publish if there is no missing component that could cause a core API refactor.

* Rename crates (replace zero-ui with something that has no hyphen). 
    - Z meaning depth opens some possibilities, unfortunately "zui" is already taken.
    - `znest`: Z-dimension (depth) + nest, Z-Nest, US pronunciation "zee nest"? 
    - `zerui`.

* Review all docs.
* Review prebuild distribution.
* Pick license and code of conduct.
* Create a GitHub user for the project?
* Create issues for each TODO.

* Publish (after all TODOs in this file resolved).
* Announce in social media.

* After publish only use pull requests.
    - We used a lot of partial commits during development.
    - Is that a problem in git history?
    - Research how other projects handled this issue.