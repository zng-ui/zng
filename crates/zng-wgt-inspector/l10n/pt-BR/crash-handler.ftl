## Debug Crash Handler

window =
    .title = {$app} - App Travou

## Panels

# save-copy-starting-name:
#     default file name
minidump =
    .open-error = Falha ao abrir minidump.
        {$error}
    .remove-error = Falha ao remover minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Salvar Copia
    .save-error = Falha ao salvar copia do minidump.
        {$error}
    .title = Minidump

panic =
    .title = Pânico

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = Timestamp: {$timestamp}
        Exit Code: {$exit_code}
        Signal: {$signal}
        Stderr: {$stderr_len -> 
                  [one] 1 byte 
                  *[other] {$stderr_len} bytes
                }
        Stdout: {$stdout_len -> 
                  [one] 1 byte 
                  *[other] {$stdout_len} bytes
                }
        Panic: {$is_panic}
        Minidump: {$minidump_path ->
                    [none] <none>
                    *[other] ${minidump_path}
                  }
        
        Args: {$args}
        OS: {$os}
    .title = Resumo

widget =
    .title = Widget
