## Debug Crash Handler

window =
    .title = {$app} - 앱이 크래시되었습니다

## Panels

# save-copy-starting-name:
#     default file name
minidump =
    .open-error = 미니덤프를 열 수 없습니다.
        {$error}
    .remove-error = 미니덤프를 삭제할 수 없습니다.
        {$error}
    .save-copy-filter-name = 미니덤프
    .save-copy-starting-name = minidump
    .save-copy-title = 복사본 저장
    .save-error = 미니덤프 복사본을 저장할 수 없습니다.
        {$error}
    .title = 미니덤프

panic =
    .title = 패닉

stderr =
    .title = 표준 오류

stdout =
    .title = 표준 출력

summary =
    .text = 타임스탬프: {$timestamp}
        종료 코드: {$exit_code}
        시그널: {$signal}
        표준 오류: {$stderr_len} 바이트
        표준 출력: {$stdout_len} 바이트
        패닉: {$is_panic}
        미니덤프: {$minidump_path}
        
        인자: {$args}
        운영체제: {$os}
    .title = 요약

widget =
    .title = 위젯