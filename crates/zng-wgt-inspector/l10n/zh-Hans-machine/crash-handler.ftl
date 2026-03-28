### Machine translated by `cargo zng l10n`, d71191013144b142e5eaea12b0fca3409e50c6d718fac39611285128494e9ee4

### 由 `cargo zng l10n` 自动生成

## 调试崩溃处理程序

window =
    .title = {$app} - 应用已崩溃

## 面板

# save-copy-starting-name:
#     默认文件名
minidump =
    .open-error = 打开小转储文件失败。
        {$error}
    .remove-error = 删除小转储文件失败。
        {$error}
    .save-copy-filter-name = 小转储文件
    .save-copy-starting-name = minidump
    .save-copy-title = 保存副本
    .save-error = 保存小转储文件副本失败。
        {$error}
    .title = 小转储文件 (Minidump)

panic =
    .title = 恐慌 (Panic)

stderr =
    .title = 标准错误 (Stderr)

stdout =
    .title = 标准输出 (Stdout)

summary =
    .text = 时间戳：{$timestamp}
        退出代码：{$exit_code}
        信号：{$signal}
        标准错误：{$stderr_len} 字节
        标准输出：{$stdout_len} 字节
        恐慌：{$is_panic}
        小转储文件：{$minidump_path}
        
        参数：{$args}
        操作系统：{$os}
    .title = 摘要

widget =
    .title = 组件 (Widget)