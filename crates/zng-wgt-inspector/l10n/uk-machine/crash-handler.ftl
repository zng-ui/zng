### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### Автоматично згенеровано `cargo zng l10n`

## Обробник аварійного завершення (Debug Crash Handler)

window =
    .title = {$app} - Програма аварійно завершила роботу

## Панелі

# save-copy-starting-name:
#     назва файлу за замовчуванням
minidump =
    .open-error = Не вдалося відкрити мінідамп (minidump).
        {$error}
    .remove-error = Не вдалося видалити мінідамп.
        {$error}
    .save-copy-filter-name = Мінідамп
    .save-copy-starting-name = minidump
    .save-copy-title = Зберегти копію
    .save-error = Не вдалося зберегти копію мінідампу.
        {$error}
    .title = Мінідамп

panic =
    .title = Паніка (Panic)

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Часова мітка: {$timestamp}
        Код виходу: {$exit_code}
        Сигнал: {$signal}
        Stderr: {$stderr_len} байтів
        Stdout: {$stdout_len} байтів
        Паніка: {$is_panic}
        Мінідамп: {$minidump_path}
        
        Аргументи: {$args}
        ОС: {$os}
    .title = Підсумок

widget =
    .title = Віджет