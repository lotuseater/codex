@echo off
setlocal EnableExtensions

pushd "%~dp0" >nul
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\build-local-codex.ps1" -Mode FastRelease
set "CODEX_BUILD_EXIT=%ERRORLEVEL%"
popd >nul

echo.
if not "%CODEX_BUILD_EXIT%"=="0" (
    echo Codex build/deploy failed with exit code %CODEX_BUILD_EXIT%.
) else (
    echo Codex build/deploy completed successfully.
)
echo.
pause
exit /b %CODEX_BUILD_EXIT%
