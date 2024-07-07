## Debug Crash Handler

window =
    .title = {$app} - App Crashed

## Panels

# save-copy-starting-name:
#     default file name
minidump =
    .open-error = Failed to open minidump.
        {$error}
    .remove-error = Failed to remove minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Save Copy
    .save-error = Failed so save minidump copy.
        {$error}
    .title = Minidump

panic =
    .title = Panic

stderr =
    .title = Stderr
    .title-plain = Stderr (plain)

stdout =
    .title = Stdout
    .title-plain = Stdout (plain)

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
    .title = Summary

widget =
    .title = Widget
