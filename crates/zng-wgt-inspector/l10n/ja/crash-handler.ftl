### machine translated, feel free to contribute corrections

window =
    .title = {$app} - アプリがクラッシュしました

minidump =
    .open-error = ミニダンプを開けませんでした。
        {$error}
    .remove-error = ミニダンプを削除できませんでした。
        {$error}
    .save-copy-filter-name = ミニダンプ
    .save-copy-starting-name = ミニダンプ
    .save-copy-title = コピーを保存
    .save-error = 失敗したため、ミニダンプのコピーを保存します。
        {$error}
    .title = ミニダンプ

panic =
    .title = パニック

stderr =
    .title = 標準エラー出力

stdout =
    .title = 標準出力

summary =
    .text = タイムスタンプ: {$timestamp}
        終了コード: {$exit_code}
        シグナル: {$signal}
        標準エラー出力: {$stderr_len ->
            [one] 1 バイト
            *[other] {$stderr_len} バイト
        }
        標準出力: {$stdout_len ->
            [one] 1 バイト
            *[other] {$stdout_len} バイト
        }
        パニック: {$is_panic}
        ミニダンプ: {$minidump_path ->
            [none] <none>
            *[other] ${minidump_path}
        }

        引数: {$args}
        OS: {$os}
    .title = 概要

widget =
    .title = ウィジェット