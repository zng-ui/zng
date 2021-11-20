# CORE

* Define a frame pass (the events, updates, layout, render and redraw that forms a frame).
* Pre-alloc memory for frame message.
* Implement event coalesce for high-pressure events, see
   Chrome: https://developers.google.com/web/updates/2017/06/aligning-input-events
   Firefox: https://bugzilla.mozilla.org/show_bug.cgi?id=1361067
   Let widgets tag their interest and then use the hit-test in view-process to decide if
   the event can be coalesced?
* Timer and resize updates are causing problems.
* Crash respawn deadlocking.

# Loop

:poll-raw:
  key_input
  char_input
  -> :update-cycle:

:update-cycle:
  ::update_events::
     key_input
     key_down
     char_input
  ::timers::
  ::update::
  ::layout::
  ::render::

  -> :poll-raw:


# Loop Idea

+:poll-raw:
|:update_events:
+:update:
 :layout:

:render:
