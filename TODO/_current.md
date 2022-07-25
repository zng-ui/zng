* Add `FrameValue` for more display items.
    - Opacity, so we don't need to convert to webrender in the app process.
    - Text color.
        - Do we only need custom reuse groups because of color here?
    - Others that cause a new frame often (border color?).

* Test integration of frame reuse with frame update.
* Merge.


* Implement virtualization, see `Optimizations.md`.
* Finish state API, see `State.md`.