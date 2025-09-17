# Used in text to insert a space between words. Usually located below the character keys.
Space = Espaço
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
Copy = Copiar
# Cut the current selection. (`APPCOMMAND_CUT`)
Cut = Recortar
# The Paste key. (`APPCOMMAND_PASTE`)
Paste = Colar
# Redo the last action. (`APPCOMMAND_REDO`)
Redo = Refazer
# Undo the last action. (`APPCOMMAND_UNDO`)
Undo = Desfazer
# Show the application’s context menu.
# This key is commonly found between the right `Super` key and the right `Ctrl` key.
ContextMenu = ≣Menu Contextual
# The `Esc` key. This key was originally used to initiate an escape sequence, but is
# now more generally used to exit or "escape" the current context, such as closing a dialog
# or exiting full screen mode.
Escape = Esc
# Open the Find dialog. (`APPCOMMAND_FIND`)
Find = 🔍Localizar
# Open a help dialog or toggle display of help information. (`APPCOMMAND_HELP`,
# `KEYCODE_HELP`)
Help = ?Ajuda
# The ZoomIn key. (`KEYCODE_ZOOM_IN`)
ZoomIn = +Aumentar Zoom
# The ZoomOut key. (`KEYCODE_ZOOM_OUT`)
ZoomOut = -Diminuir Zoom
# The Brightness Down key. Typically controls the display brightness.
# (`KEYCODE_BRIGHTNESS_DOWN`)
BrightnessDown = Diminuir Brilho
# The Brightness Up key. Typically controls the display brightness. (`KEYCODE_BRIGHTNESS_UP`)
BrightnessUp = Aumentar Brilho
# Toggle removable media to eject (open) and insert (close) state. (`KEYCODE_MEDIA_EJECT`)
Eject = ⏏Ejetar

# Close the current document or message (Note: This doesn’t close the application).
# (`APPCOMMAND_CLOSE`)
Close = Fechar
# Open an editor to forward the current message. (`APPCOMMAND_FORWARD_MAIL`)
MailForward = Encaminhar E-mail
# Open an editor to reply to the current message. (`APPCOMMAND_REPLY_TO_MAIL`)
MailReply = Responder E-mail
# Send the current message. (`APPCOMMAND_SEND_MAIL`)
MailSend = Enviar E-mail
# Close the current media, for example to close a CD or DVD tray. (`KEYCODE_MEDIA_CLOSE`)
MediaClose = Fechar Media
# Initiate or continue forward playback at faster than normal speed, or increase speed if
# already fast forwarding. (`APPCOMMAND_MEDIA_FAST_FORWARD`, `KEYCODE_MEDIA_FAST_FORWARD`)
MediaFastForward = ⏩Avançar Rápido
# Pause the currently playing media. (`APPCOMMAND_MEDIA_PAUSE`, `KEYCODE_MEDIA_PAUSE`)
#
# Note: Media controller devices should use this value rather than `"Pause"` for their pause
# keys.
MediaPause = ⏸Pausar
# Initiate or continue media playback at normal speed, if not currently playing at normal
# speed. (`APPCOMMAND_MEDIA_PLAY`, `KEYCODE_MEDIA_PLAY`)
MediaPlay = ▶Play
# Toggle media between play and pause states. (`APPCOMMAND_MEDIA_PLAY_PAUSE`,
# `KEYCODE_MEDIA_PLAY_PAUSE`)
MediaPlayPause = ⏯Play/Pausar
# Initiate or resume recording of currently selected media. (`APPCOMMAND_MEDIA_RECORD`,
# `KEYCODE_MEDIA_RECORD`)
MediaRecord = ⏺Gravar
# Initiate or continue reverse playback at faster than normal speed, or increase speed if
# already rewinding. (`APPCOMMAND_MEDIA_REWIND`, `KEYCODE_MEDIA_REWIND`)
MediaRewind = ⏪Rebobinar
# Stop media playing, pausing, forwarding, rewinding, or recording, if not already stopped.
# (`APPCOMMAND_MEDIA_STOP`, `KEYCODE_MEDIA_STOP`)
MediaStop = ⏹Parar
# Seek to next media or program track. (`APPCOMMAND_MEDIA_NEXTTRACK`, `KEYCODE_MEDIA_NEXT`)
MediaTrackNext = ⏭Próxima Faixa
# Seek to previous media or program track. (`APPCOMMAND_MEDIA_PREVIOUSTRACK`,
# `KEYCODE_MEDIA_PREVIOUS`)
MediaTrackPrevious = ⏮Faixa Anterior
# Open a new document or message. (`APPCOMMAND_NEW`)
New = Novo
# Open an existing document or message. (`APPCOMMAND_OPEN`)
Open = Abrir
# Print the current document or message. (`APPCOMMAND_PRINT`)
Print = Imprimir
# Save the current document or message. (`APPCOMMAND_SAVE`)
Save = Salvar

# Decrease audio volume. (`APPCOMMAND_VOLUME_DOWN`, `KEYCODE_VOLUME_DOWN`)
AudioVolumeDown = -Diminuir Volume
# Increase audio volume. (`APPCOMMAND_VOLUME_UP`, `KEYCODE_VOLUME_UP`)
AudioVolumeUp = +Aumentar Volume
# Toggle between muted state and prior volume level. (`APPCOMMAND_VOLUME_MUTE`,
# `KEYCODE_VOLUME_MUTE`)
AudioVolumeMute = Silenciar
# Toggle the microphone on/off. (`APPCOMMAND_MIC_ON_OFF_TOGGLE`)
MicrophoneToggle = Mic On/Off
# Decrease microphone volume. (`APPCOMMAND_MICROPHONE_VOLUME_DOWN`)
MicrophoneVolumeDown = Diminuir Volume do Mic
# Increase microphone volume. (`APPCOMMAND_MICROPHONE_VOLUME_UP`)
MicrophoneVolumeUp = Aumentar Volume do Mic
# Mute the microphone. (`APPCOMMAND_MICROPHONE_VOLUME_MUTE`, `KEYCODE_MUTE`)
MicrophoneVolumeMute = Silenciar Mic
# The first generic "LaunchApplication" key. This is commonly associated with launching "My
# Computer", and may have a computer symbol on the key. (`APPCOMMAND_LAUNCH_APP1`)
LaunchApplication1 = App 1
# The second generic "LaunchApplication" key. This is commonly associated with launching
# "Calculator", and may have a calculator symbol on the key. (`APPCOMMAND_LAUNCH_APP2`,
# `KEYCODE_CALCULATOR`)
LaunchApplication2 = App 2
# The "Calendar" key. (`KEYCODE_CALENDAR`)
LaunchCalendar = Calendário
# The "Contacts" key. (`KEYCODE_CONTACTS`)
LaunchContacts = Contatos
# The "Mail" key. (`APPCOMMAND_LAUNCH_MAIL`)
LaunchMail = E-mail
# The "Phone" key.
LaunchPhone = Telefone
# The "Excel" key.
LaunchSpreadsheet = Planilha
# The "Web Browser" key.
LaunchWebBrowser = Navegador
# Navigate to previous content or page in current history. (`APPCOMMAND_BROWSER_BACKWARD`)
BrowserBack = ←Retornar
# Open the list of browser favorites. (`APPCOMMAND_BROWSER_FAVORITES`)
BrowserFavorites = Favoritos
# Navigate to next content or page in current history. (`APPCOMMAND_BROWSER_FORWARD`)
BrowserForward = →Avançar
# Go to the user’s preferred home page. (`APPCOMMAND_BROWSER_HOME`)
BrowserHome = ⌂Página Inicial
# Refresh the current page or content. (`APPCOMMAND_BROWSER_REFRESH`)
BrowserRefresh = ⟳Atualizar