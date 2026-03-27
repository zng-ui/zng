### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### Được tạo tự động bởi `cargo zng l10n`

## Trình xử lý sự cố gỡ lỗi

window =
    .title = {$app} - Ứng dụng đã gặp sự cố

## Các bảng điều khiển

# save-copy-starting-name:
#     tên tệp mặc định
minidump =
    .open-error = Không thể mở minidump.
        {$error}
    .remove-error = Không thể xóa minidump.
        {$error}
    .save-copy-filter-name = Minidump
    .save-copy-starting-name = minidump
    .save-copy-title = Lưu bản sao
    .save-error = Không thể lưu bản sao minidump.
        {$error}
    .title = Minidump

panic =
    .title = Panic (Lỗi nghiêm trọng)

stderr =
    .title = Stderr (Luồng lỗi tiêu chuẩn)

stdout =
    .title = Stdout (Luồng xuất tiêu chuẩn)

summary =
    .text = Dấu thời gian: {$timestamp}
        Mã thoát: {$exit_code}
        Tín hiệu: {$signal}
        Stderr: {$stderr_len} byte
        Stdout: {$stdout_len} byte
        Panic: {$is_panic}
        Minidump: {$minidump_path}
        
        Đối số: {$args}
        Hệ điều hành: {$os}
    .title = Tóm tắt

widget =
    .title = Widget