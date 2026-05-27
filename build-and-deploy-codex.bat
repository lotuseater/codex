@echo off
setlocal

set "SCRIPT_DIR=%~dp0"
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%SCRIPT_DIR%scripts\build-local-codex.ps1" -Mode FastRelease %*
set "EXIT_CODE=%ERRORLEVEL%"

echo.
if not "%EXIT_CODE%"=="0" (
    echo build-and-deploy-codex failed with exit code %EXIT_CODE%.
) else (
    echo build-and-deploy-codex completed successfully.
)
if /I not "%CODEX_BUILD_NO_PAUSE%"=="1" pause
exit /b %EXIT_CODE%
