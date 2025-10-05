@echo off
setlocal
set PS1="%~dp0run_kernel_gui.ps1"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File %PS1% %*
exit /b %ERRORLEVEL%

