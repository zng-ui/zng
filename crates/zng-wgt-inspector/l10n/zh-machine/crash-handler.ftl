## 调试崩溃处理器

window =
    .title = {$app} - 应用崩溃

## 面板

# save-copy-starting-name:
#     默认文件名
minidump =
    .open-error = 打开最小转储失败。
        {$error}
    .remove-error = 删除最小转储失败。
        {$error}
    .save-copy-filter-name = 最小转储
    .save-copy-starting-name = minidump
    .save-copy-title = 保存副本
    .save-error = 保存最小转储副本失败。
        {$error}
    .title = 最小转储

panic =
    .title = Panic

stderr =
    .title = 标准错误

stdout =
    .title = 标准输出

summary =
    .text = 时间戳: {$timestamp}
        退出码: {$exit_code}
        信号: {$signal}
        标准错误: {$stderr_len} 字节
        标准输出: {$stdout_len} 字节
        Panic: {$is_panic}
        最小转储: {$minidump_path}
        
        参数: {$args}
        操作系统: {$os}
    .title = 摘要

widget =
    .title = 小部件