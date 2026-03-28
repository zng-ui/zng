### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### `cargo zng l10n` द्वारा स्वतः जनरेट किया गया

## डिबग क्रैश हैंडलर

window =
    .title = {$app} - ऐप क्रैश हो गया

## पैनल्स

# save-copy-starting-name:
#     डिफ़ॉल्ट फ़ाइल का नाम
minidump =
    .open-error = मिनीडंप (minidump) खोलने में विफल।
        {$error}
    .remove-error = मिनीडंप को हटाने में विफल।
        {$error}
    .save-copy-filter-name = मिनीडंप
    .save-copy-starting-name = मिनीडंप
    .save-copy-title = कॉपी सेव करें
    .save-error = मिनीडंप की कॉपी सेव करने में विफल।
        {$error}
    .title = मिनीडंप

panic =
    .title = पैनिक (Panic)

stderr =
    .title = स्टडर (Stderr)

stdout =
    .title = स्टडआउट (Stdout)

summary =
    .text = टाइमस्टैम्प: {$timestamp}
        एग्जिट कोड: {$exit_code}
        सिग्नल: {$signal}
        स्टडर: {$stderr_len} बाइट्स
        स्टडआउट: {$stdout_len} बाइट्स
        पैनिक: {$is_panic}
        मिनीडंप: {$minidump_path}
        
        आर्ग्यूमेंट्स: {$args}
        ओएस: {$os}
    .title = सारांश

widget =
    .title = विजेट