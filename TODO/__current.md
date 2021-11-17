# CORE

* Define a frame pass (the events, updates, layout, render and redraw that forms a frame).
* Implement event coalesce for high-pressure events, see
   Chrome: https://developers.google.com/web/updates/2017/06/aligning-input-events
   Firefox: https://bugzilla.mozilla.org/show_bug.cgi?id=1361067
   Let widgets tag their interest and then use the hit-test in view-process to decide if
   the event can be coalesced?
* Timer and resize updates are causing problems.
* Slow frame upload (2ms for 2mb)?
* Crash respawn deadlocking.