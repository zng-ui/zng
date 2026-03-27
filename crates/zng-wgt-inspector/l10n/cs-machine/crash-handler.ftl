### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### Automaticky vygenerováno pomocí `cargo zng l10n`

## Debug Crash Handler

window =
    .title = {$app} – Aplikace havarovala

## Panely

# save-copy-starting-name:
#     výchozí název souboru
minidump =
    .open-error = Nepodařilo se otevřít minidump.
        {$error}
    .remove-error = Nepodařilo se odstranit minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Uložit kopii
    .save-error = Nepodařilo se uložit kopii minidumpu.
        {$error}
    .title = Minidump

panic =
    .title = Panika (Panic)

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Časové razítko: {$timestamp}
        Kód ukončení: {$exit_code}
        Signál: {$signal}
        Stderr: {$stderr_len} bajtů
        Stdout: {$stdout_len} bajtů
        Panika: {$is_panic}
        Minidump: {$minidump_path}
        
        Argumenty: {$args}
        OS: {$os}
    .title = Shrnutí

widget =
    .title = Widget