* Image requests a render each layout, optimize.
* Detect frame sent when another is still rendering (block/log?)
* Implement event coalesce for high-pressure events, see
   Chrome: https://developers.google.com/web/updates/2017/06/aligning-input-events
   Firefox: https://bugzilla.mozilla.org/show_bug.cgi?id=1361067
* Implement `rust_analyzer_check` cancellation.