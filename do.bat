@echo off

:: Bypass "Terminate Batch Job" prompt.
if "%~1"=="-FIXED_CTRL_C" (
   :: Remove the -FIXED_CTRL_C parameter
   shift
) else (
   :: Run the batch with <nul and -FIXED_CTRL_C
   call <nul %0 -FIXED_CTRL_C %*
   goto :EOF
)

:: Collect Arguments
set ARGS=
:next
if "%1"=="" goto done
set ARGS=%ARGS% %1
shift
goto next
:done

:: Run Task
rustc do-tasks.rs --out-dir "target/do-tasks"
"target/do-tasks/do-tasks.exe" %ARGS%