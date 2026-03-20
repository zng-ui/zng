## Debug-Crash-Handler

window =
    .title = {$app} - App abgestürzt

## Panels

# save-copy-starting-name:
#     Standard-Dateiname
minidump =
    .open-error = Minidump konnte nicht geöffnet werden.
        {$error}
    .remove-error = Minidump konnte nicht entfernt werden.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Kopie speichern
    .save-error = Minidump-Kopie konnte nicht gespeichert werden.
        {$error}
    .title = Minidump

panic =
    .title = Panic

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Zeitstempel: {$timestamp}
        Exit-Code: {$exit_code}
        Signal: {$signal}
        Stderr: {$stderr_len} Bytes
        Stdout: {$stdout_len} Bytes
        Panic: {$is_panic}
        Minidump: {$minidump_path}
        
        Argumente: {$args}
        Betriebssystem: {$os}
    .title = Zusammenfassung

widget =
    .title = Widget