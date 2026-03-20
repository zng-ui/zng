## Gestore Crash Debug

window =
    .title = {$app} - App Arrestata

## Pannelli

# save-copy-starting-name:
#     nome file predefinito
minidump =
    .open-error = Impossibile aprire il minidump.
        {$error}
    .remove-error = Impossibile rimuovere il minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Salva Copia
    .save-error = Impossibile salvare la copia del minidump.
        {$error}
    .title = Minidump

panic =
    .title = Panic

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Timestamp: {$timestamp}
        Codice di uscita: {$exit_code}
        Segnale: {$signal}
        Stderr: {$stderr_len} byte
        Stdout: {$stdout_len} byte
        Panic: {$is_panic}
        Minidump: {$minidump_path}
        
        Argomenti: {$args}
        OS: {$os}
    .title = Riepilogo

widget =
    .title = Widget