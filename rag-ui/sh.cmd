@echo off
REM Wrapper so Trunk's built-in hooks that call "sh -c "command"" work on Windows.
REM Trunk passes: sh -c "npx tailwindcss ..." so we run only the command (%~2).
if "%~1"=="-c" (cmd /c %~2) else (cmd /c %*)
