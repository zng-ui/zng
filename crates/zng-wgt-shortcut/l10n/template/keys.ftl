# Gesture key names
#
# Only valid gesture keys are included, modifier and composition keys are not included.
# Also not included `Key::Char`, `Key::Str` and `Key::Undefined`.
#
# * The ID is the `Key` variant name.
# * A OS generic text must be provided, optional OS specific text can be set as attributes.
# * OS attribute is a `std::env::consts::OS` value.
#
# This file does not include all valid keys, just the more common ones that are known to be localized in some languages

### Comments copied from the `Key` enum

# The `Enter` or `↵` key. Used to activate current selection or accept current input. This key
# value is also used for the `Return` (Macintosh numpad) key. This key value is also used for
# the Android `KEYCODE_DPAD_CENTER`.
Enter = ↵Enter
    .macos = ↵Return
# The Horizontal Tabulation `Tab` key.
Tab = ⭾Tab
# Used in text to insert a space between words. Usually located below the character keys.
Space = Space
# Navigate or traverse downward. (`KEYCODE_DPAD_DOWN`)
ArrowDown = ↓
# Navigate or traverse leftward. (`KEYCODE_DPAD_LEFT`)
ArrowLeft = ←
# Navigate or traverse rightward. (`KEYCODE_DPAD_RIGHT`)
ArrowRight = →
# Navigate or traverse upward. (`KEYCODE_DPAD_UP`)
ArrowUp = ↑
# The End key, used with keyboard entry to go to the end of content (`KEYCODE_MOVE_END`).
End = End
# The Home key, used with keyboard entry, to go to start of content (`KEYCODE_MOVE_HOME`).
# For the mobile phone `Home` key (which goes to the phone’s main screen), use [`GoHome`].
Home = Home
# Scroll down or display next page of content.
PageDown = PgDn
# Scroll up or display previous page of content.
PageUp = PgUp
# Used to remove the character to the left of the cursor. This key value is also used for
# the key labelled `Delete` on MacOS keyboards.
Backspace = ←Backspace
    .macos = Delete
# Copy the current selection. (`APPCOMMAND_COPY`)
Copy = Copy
# Cut the current selection. (`APPCOMMAND_CUT`)
Cut = Cut
# Used to delete the character to the right of the cursor. This key value is also used for the
# key labelled `Delete` on MacOS keyboards when `Fn` is active.
Delete = Delete
    .macos = Forward Delete 
# Toggle between text modes for insertion or overtyping.
# (`KEYCODE_INSERT`)
Insert = Insert
# The Paste key. (`APPCOMMAND_PASTE`)
Paste = Paste
# Redo the last action. (`APPCOMMAND_REDO`)
Redo = Redo
# Undo the last action. (`APPCOMMAND_UNDO`)
Undo = Undo
# Show the application’s context menu.
# This key is commonly found between the right `Super` key and the right `Ctrl` key.
ContextMenu = ≣Context Menu
# The `Esc` key. This key was originally used to initiate an escape sequence, but is
# now more generally used to exit or "escape" the current context, such as closing a dialog
# or exiting full screen mode.
Escape = Esc
# Open the Find dialog. (`APPCOMMAND_FIND`)
Find = 🔍Find
# Open a help dialog or toggle display of help information. (`APPCOMMAND_HELP`,
# `KEYCODE_HELP`)
Help = ?Help
# The ZoomIn key. (`KEYCODE_ZOOM_IN`)
ZoomIn = +Zoom In
# The ZoomOut key. (`KEYCODE_ZOOM_OUT`)
ZoomOut = -Zoom Out
# The Brightness Down key. Typically controls the display brightness.
# (`KEYCODE_BRIGHTNESS_DOWN`)
BrightnessDown = Brightness Down
# The Brightness Up key. Typically controls the display brightness. (`KEYCODE_BRIGHTNESS_UP`)
BrightnessUp = Brightness Up
# Toggle removable media to eject (open) and insert (close) state. (`KEYCODE_MEDIA_EJECT`)
Eject = ⏏Eject
# Log-off key.
LogOff = Log-off
# Initiate print-screen function.
PrintScreen = PrtSc

# Close the current document or message (Note: This doesn’t close the application).
# (`APPCOMMAND_CLOSE`)
Close = Close
# Open an editor to forward the current message. (`APPCOMMAND_FORWARD_MAIL`)
MailForward = Forward Mail
# Open an editor to reply to the current message. (`APPCOMMAND_REPLY_TO_MAIL`)
MailReply = Reply Mail
# Send the current message. (`APPCOMMAND_SEND_MAIL`)
MailSend = Send Mail
# Close the current media, for example to close a CD or DVD tray. (`KEYCODE_MEDIA_CLOSE`)
MediaClose = Close Media
# Initiate or continue forward playback at faster than normal speed, or increase speed if
# already fast forwarding. (`APPCOMMAND_MEDIA_FAST_FORWARD`, `KEYCODE_MEDIA_FAST_FORWARD`)
MediaFastForward = ⏩Fast Forward
# Pause the currently playing media. (`APPCOMMAND_MEDIA_PAUSE`, `KEYCODE_MEDIA_PAUSE`)
#
# Note: Media controller devices should use this value rather than `"Pause"` for their pause
# keys.
MediaPause = ⏸Pause
# Initiate or continue media playback at normal speed, if not currently playing at normal
# speed. (`APPCOMMAND_MEDIA_PLAY`, `KEYCODE_MEDIA_PLAY`)
MediaPlay = ▶Play
# Toggle media between play and pause states. (`APPCOMMAND_MEDIA_PLAY_PAUSE`,
# `KEYCODE_MEDIA_PLAY_PAUSE`)
MediaPlayPause = ⏯Play/Pause
# Initiate or resume recording of currently selected media. (`APPCOMMAND_MEDIA_RECORD`,
# `KEYCODE_MEDIA_RECORD`)
MediaRecord = ⏺Record
# Initiate or continue reverse playback at faster than normal speed, or increase speed if
# already rewinding. (`APPCOMMAND_MEDIA_REWIND`, `KEYCODE_MEDIA_REWIND`)
MediaRewind = ⏪Rewind
# Stop media playing, pausing, forwarding, rewinding, or recording, if not already stopped.
# (`APPCOMMAND_MEDIA_STOP`, `KEYCODE_MEDIA_STOP`)
MediaStop = ⏹Stop
# Seek to next media or program track. (`APPCOMMAND_MEDIA_NEXTTRACK`, `KEYCODE_MEDIA_NEXT`)
MediaTrackNext = ⏭Next Track
# Seek to previous media or program track. (`APPCOMMAND_MEDIA_PREVIOUSTRACK`,
# `KEYCODE_MEDIA_PREVIOUS`)
MediaTrackPrevious = ⏮Previous Track
# Open a new document or message. (`APPCOMMAND_NEW`)
New = New
# Open an existing document or message. (`APPCOMMAND_OPEN`)
Open = Open
# Print the current document or message. (`APPCOMMAND_PRINT`)
Print = Print
# Save the current document or message. (`APPCOMMAND_SAVE`)
Save = Save

# Decrease audio volume. (`APPCOMMAND_VOLUME_DOWN`, `KEYCODE_VOLUME_DOWN`)
AudioVolumeDown = -Volume Down
# Increase audio volume. (`APPCOMMAND_VOLUME_UP`, `KEYCODE_VOLUME_UP`)
AudioVolumeUp = +Volume Up
# Toggle between muted state and prior volume level. (`APPCOMMAND_VOLUME_MUTE`,
# `KEYCODE_VOLUME_MUTE`)
AudioVolumeMute = Mute
# Toggle the microphone on/off. (`APPCOMMAND_MIC_ON_OFF_TOGGLE`)
MicrophoneToggle = Mic On/Off
# Decrease microphone volume. (`APPCOMMAND_MICROPHONE_VOLUME_DOWN`)
MicrophoneVolumeDown = Mic Volume Down
# Increase microphone volume. (`APPCOMMAND_MICROPHONE_VOLUME_UP`)
MicrophoneVolumeUp = Mic Volume Up
# Mute the microphone. (`APPCOMMAND_MICROPHONE_VOLUME_MUTE`, `KEYCODE_MUTE`)
MicrophoneVolumeMute = Mic Mute
# The first generic "LaunchApplication" key. This is commonly associated with launching "My
# Computer", and may have a computer symbol on the key. (`APPCOMMAND_LAUNCH_APP1`)
LaunchApplication1 = App 1
# The second generic "LaunchApplication" key. This is commonly associated with launching
# "Calculator", and may have a calculator symbol on the key. (`APPCOMMAND_LAUNCH_APP2`,
# `KEYCODE_CALCULATOR`)
LaunchApplication2 = App 2
# The "Calendar" key. (`KEYCODE_CALENDAR`)
LaunchCalendar = Calendar
# The "Contacts" key. (`KEYCODE_CONTACTS`)
LaunchContacts = Contacts
# The "Mail" key. (`APPCOMMAND_LAUNCH_MAIL`)
LaunchMail = Mail
# The "Media Player" key. (`APPCOMMAND_LAUNCH_MEDIA_SELECT`)
LaunchMediaPlayer = Media Player
# The "Music Player" key.
LaunchMusicPlayer = Music Player
# The "Phone" key.
LaunchPhone = Phone
# The "Screen Saver" key.
LaunchScreenSaver = Screen Saver
# The "Excel" key.
LaunchSpreadsheet = Spreadsheet
# The "Web Browser" key.
LaunchWebBrowser = Browser
# The "Webcam" key.
LaunchWebCam = Web Cam
# The "Word" key.
LaunchWordProcessor = Word Processor
# Navigate to previous content or page in current history. (`APPCOMMAND_BROWSER_BACKWARD`)
BrowserBack = ←Back
# Open the list of browser favorites. (`APPCOMMAND_BROWSER_FAVORITES`)
BrowserFavorites = Favorites
# Navigate to next content or page in current history. (`APPCOMMAND_BROWSER_FORWARD`)
BrowserForward = →Forward
# Go to the user’s preferred home page. (`APPCOMMAND_BROWSER_HOME`)
BrowserHome = ⌂Home
# Refresh the current page or content. (`APPCOMMAND_BROWSER_REFRESH`)
BrowserRefresh = ⟳Refresh