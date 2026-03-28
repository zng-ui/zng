### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### Generato automaticamente da `cargo zng l10n`

## Gestore crash di debug

window =
    .title = {$app} - L'app è andata in crash

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
    .save-copy-title = Salva copia
    .save-error = Impossibile salvare la copia del minidump.
        {$error}
    .title = Minidump

panic =
    .title = Panico

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
        Panico: {$is_panic}
        Minidump: {$minidump_path}
        
        Argomenti: {$args}
        OS: {$os}
    .title = Riepilogo

widget =
    .title = Widget