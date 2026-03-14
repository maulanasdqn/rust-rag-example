@echo off
if "%~1"=="-c" (cmd /c %~2) else (cmd /c %*)
