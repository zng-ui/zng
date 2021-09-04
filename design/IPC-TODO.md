* Finish `resize.md`
* Implement and test window respawn.
* Implement fast detection of view-process crash.
* Don't respawn for some exit codes? Shutdown by Task Manager exits with "1" we don't want
  to respawn in this case.


       App                    | |                        View
  ----------------------------|-|--------------------------------------------------------
         OneShotNamed         |>| "Connect with App using the channel name"
  "Receive using OneShotNamed"|<| (RequestSender, EventReceiver, ResponseChannelSender)
        ResponseSender        |>| "Receive ResponseSender using ResponseChannelReceiver"
               