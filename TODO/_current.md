* Add `FrameValue` for more display items.
    - Opacity, so we don't need to convert to webrender in the app process.
    - Text color.
        - Do we only need custom reuse groups because of color here?
    - Others that cause a new frame often (border color?).

* Review reuse after frame update.


* Implement virtualization, see `Optimizations.md`.
* Finish state API, see `State.md`.