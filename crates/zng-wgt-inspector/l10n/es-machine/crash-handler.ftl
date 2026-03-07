### machine translated, feel free to contribute corrections

window =
    .title = {$app} - Aplicación Bloqueada

minidump =
    .open-error = Error al abrir el minivolcado.
        {$error}
    .remove-error = Error al eliminar el minivolcado.
        {$error}
    .save-copy-filter-name = Minivolcado
    .save-copy-starting-name = minivolcado
    .save-copy-title = Guardar copia
    .save-error = Error al guardar la copia del minivolcado.
        {$error}
.title = Minivolcado

panic =
    .title = Pánico

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Marca de tiempo: {$timestamp}
        Código de salida: {$exit_code}
        Señal: {$signal}
        Stderr: {$stderr_len} bytes
        Stdout: {$stdout_len} bytes
        Pánico: {$is_panic}
        Minivolcado: {$minidump_path}

        Argumentos: {$args}
        SO: {$os}
    .title = Resumen

widget =
    .title = Widget