* Refactor IPC to be async, don't expect `()` responses?
* Detect frame sent when another is still rendering (block/log?)
* Implement `rust_analyzer_check` cancellation.
* Implement event coalesce for high-pressure events, see
   Chrome: https://developers.google.com/web/updates/2017/06/aligning-input-events
   Firefox: https://bugzilla.mozilla.org/show_bug.cgi?id=1361067
   Let widgets tag their interest and then use the hit-test in view-process to decide if
   the event can be coalesced.
* Check FocusManager and window maximized at startup.