### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### Auto generat per `cargo zng l10n`

## Gestor d'errades de depuració

window =
    .title = {$app} - L'aplicació ha fallat

## Panells

# save-copy-starting-name:
#     nom de fitxer per defecte
minidump =
    .open-error = No s'ha pogut obrir el minidump.
        {$error}
    .remove-error = No s'ha pogut eliminar el minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Desa una còpia
    .save-error = No s'ha pogut desar la còpia del minidump.
        {$error}
    .title = Minidump

panic =
    .title = Pànic

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Marca de temps: {$timestamp}
        Codi de sortida: {$exit_code}
        Senyal: {$signal}
        Stderr: {$stderr_len} bytes
        Stdout: {$stdout_len} bytes
        Pànic: {$is_panic}
        Minidump: {$minidump_path}
        
        Arguments: {$args}
        SO: {$os}
    .title = Resum

widget =
    .title = Giny