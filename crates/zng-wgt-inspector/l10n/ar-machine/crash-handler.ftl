### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### تم إنشاؤه تلقائيًا بواسطة `cargo zng l10n`

## معالج انهيار البرنامج (Debug Crash Handler)

window =
    .title = {$app} - تعطل التطبيق

## اللوحات (Panels)

# save-copy-starting-name:
#     اسم الملف الافتراضي
minidump =
    .open-error = فشل فتح ملف minidump.
        {$error}
    .remove-error = فشل حذف ملف minidump.
        {$error}
    .save-copy-filter-name = ملف Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = حفظ نسخة
    .save-error = فشل حفظ نسخة من ملف minidump.
        {$error}
    .title = Minidump

panic =
    .title = الذعر (Panic)

stderr =
    .title = مخرج الخطأ (Stderr)

stdout =
    .title = المخرج القياسي (Stdout)

summary =
    .text = الطابع الزمني: {$timestamp}
        رمز الخروج: {$exit_code}
        الإشارة: {$signal}
        مخرج الخطأ (Stderr): {$stderr_len} بايت
        المخرج القياسي (Stdout): {$stdout_len} بايت
        الذعر (Panic): {$is_panic}
        ملف Minidump: {$minidump_path}
        
        الوسائط: {$args}
        نظام التشغيل: {$os}
    .title = الملخص

widget =
    .title = الأداة (Widget)