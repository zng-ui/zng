## Manejador de Fallos de Depuración

window =
    .title = {$app} - La aplicación ha fallado

## Paneles

# save-copy-starting-name:
#     nombre de archivo predeterminado
minidump =
    .open-error = Error al abrir el minidump.
        {$error}
    .remove-error = Error al eliminar el minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Guardar copia
    .save-error = Error al guardar la copia del minidump.
        {$error}
    .title = Minidump

panic =
    .title = Panic

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
        Panic: {$is_panic}
        Minidump: {$minidump_path}
        
        Args: {$args}
        SO: {$os}
    .title = Resumen

widget =
    .title = Widget