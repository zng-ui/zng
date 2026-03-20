## デバッグクラッシュハンドラ

window =
    .title = {$app} - アプリがクラッシュしました

## パネル

# save-copy-starting-name:
#     デフォルトのファイル名
minidump =
    .open-error = ミニダンプを開けませんでした。
        {$error}
    .remove-error = ミニダンプを削除できませんでした。
        {$error}
    .save-copy-filter-name = ミニダンプ
    .save-copy-starting-name = minidump
    .save-copy-title = コピーを保存
    .save-error = ミニダンプのコピーを保存できませんでした。
        {$error}
    .title = ミニダンプ

panic =
    .title = パニック

stderr =
    .title = 標準エラー

stdout =
    .title = 標準出力

summary =
    .text = タイムスタンプ: {$timestamp}
        終了コード: {$exit_code}
        シグナル: {$signal}
        標準エラー: {$stderr_len} バイト
        標準出力: {$stdout_len} バイト
        パニック: {$is_panic}
        ミニダンプ: {$minidump_path}
        
        引数: {$args}
        OS: {$os}
    .title = サマリー

widget =
    .title = ウィジェット