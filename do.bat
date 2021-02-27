@echo off
set errorlevel=0

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
set DO_TASK_OUT=target\do-tasks
set DO_TASK_EXE=%DO_TASK_OUT%\do-tasks.exe

if not exist %DO_TASK_EXE% (
   rustc do-tasks.rs --edition 2018 --out-dir %DO_TASK_OUT% -C opt-level=3
)
if %errorlevel% == 0 (
   %DO_TASK_EXE% %ARGS%
)