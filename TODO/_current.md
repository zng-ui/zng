* Review `Text!` var subscriptions, right now `resolve_text` subscribes to all vars.
    - This ensures that the variable is subscribed only once.
    - But now that we have direct updates we should move subscriptions to nodes that apply the vars.
* Test all.
* Clean, update.
    - Webrender for Fx113?
    - Only 3 days to stable release.

* Finish test edit & selection.
    - No char event is emitted for tab?
    - Implement cursor position.
    - Implement selection.

* Implement localization.
    - Similar to `CONFIG`, maybe even backed by it.
    - Review localization standard formats.
        - Translators use an app to edit?