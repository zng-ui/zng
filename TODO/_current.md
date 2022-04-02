* Example gradient breaks when restoring from a maximize (although it works if the window was resized first)
  * the bug is being caused by apply_layout being called infinitely (the reason for THAT not being known at the moment)
  * this bug appears related to monitor size as well, as it's only happening in 1920x1080 and only if the taskbar isn't hidden.
  * it's worth noting that loop polling limit at zero-ui-core/src/app.rs (line 1378) is not being set off by this infinite loop.