# CORE

* Timer and resize updates are causing problems.
* Implement event coalesce for high-pressure events, see
   Chrome: https://developers.google.com/web/updates/2017/06/aligning-input-events
   Firefox: https://bugzilla.mozilla.org/show_bug.cgi?id=1361067
   Let widgets tag their interest and then use the hit-test in view-process to decide if
   the event can be coalesced?
* Slow frame upload (2ms for 2mb)?
* Crash respawn deadlocking.

# DO

* Implement `rust_analyzer_check` cancellation.