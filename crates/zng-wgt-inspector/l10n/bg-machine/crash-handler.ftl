### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### Автоматично генерирано от `cargo zng l10n`

## Debug Crash Handler

window =
    .title = {$app} - Приложението се срина

## Panels

# save-copy-starting-name:
#     default file name
minidump =
    .open-error = Неуспешно отваряне на minidump.
        {$error}
    .remove-error = Неуспешно премахване на minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Запазване на копие
    .save-error = Неуспешно запазване на копие на minidump.
        {$error}
    .title = Minidump

panic =
    .title = Panic (Авария)

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Времево клеймо: {$timestamp}
        Код на излизане: {$exit_code}
        Сигнал: {$signal}
        Stderr: {$stderr_len} байта
        Stdout: {$stdout_len} байта
        Panic: {$is_panic}
        Minidump: {$minidump_path}
        
        Аргументи: {$args}
        Операционна система: {$os}
    .title = Резюме

widget =
    .title = Уиджети