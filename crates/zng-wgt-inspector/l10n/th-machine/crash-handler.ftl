### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### สร้างโดยอัตโนมัติโดย `cargo zng l10n`

## โปรแกรมจัดการการหยุดทำงานกะทันหัน (Debug Crash Handler)

window =
    .title = {$app} - แอปหยุดทำงานกะทันหัน

## แผงควบคุม (Panels)

# save-copy-starting-name:
#     ชื่อไฟล์เริ่มต้น
minidump =
    .open-error = ไม่สามารถเปิด Minidump ได้
        {$error}
    .remove-error = ไม่สามารถลบ Minidump ได้
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = บันทึกสำเนา
    .save-error = ไม่สามารถบันทึกสำเนา Minidump ได้
        {$error}
    .title = Minidump

panic =
    .title = Panic

stderr =
    .title = Stderr

stdout =
    .title = Stdout

summary =
    .text = เวลา: {$timestamp}
        รหัสการออก: {$exit_code}
        สัญญาณ: {$signal}
        Stderr: {$stderr_len} ไบต์
        Stdout: {$stdout_len} ไบต์
        Panic: {$is_panic}
        Minidump: {$minidump_path}
        
        อาร์กิวเมนต์: {$args}
        ระบบปฏิบัติการ: {$os}
    .title = สรุป

widget =
    .title = วิดเจ็ต