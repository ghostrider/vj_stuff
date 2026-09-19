@echo off
rem Copies the built FFGL plugin DLLs to Resolume's Extra Effects folder.
rem
rem Usage: install.bat [Release|Debug] [target-folder]
rem   Release|Debug   build configuration to copy (default: Release)
rem   target-folder   overrides the Resolume folder below
rem
rem Copies our own plugins (ffgl-plugins\*\bin\x64\<config>\*.dll) and the SDK example
rem plugins (external\ffgl\binaries\x64\<config>\*.dll). Restart Resolume afterwards;
rem it will not overwrite a DLL that a running instance has already loaded.

setlocal
set "CONFIG=%~1"
if "%CONFIG%"=="" set "CONFIG=Release"
set "TARGET=%~2"
if "%TARGET%"=="" set "TARGET=%USERPROFILE%\Documents\Resolume Arena\Extra Effects"

set "HERE=%~dp0"
set "SDK_BIN=%HERE%..\external\ffgl\binaries\x64\%CONFIG%"

if not exist "%TARGET%\" (
    echo Target folder does not exist: %TARGET%
    exit /b 1
)

set COPIED=0
set FAILED=0

for /d %%P in ("%HERE%*") do call :copy_dir "%%P\bin\x64\%CONFIG%"
call :copy_dir "%SDK_BIN%"

echo.
echo %COPIED% copied, %FAILED% failed  ->  %TARGET%
if %FAILED% gtr 0 (
    echo Failed copies are usually DLLs locked by a running Resolume - close it and retry.
    exit /b 1
)
if %COPIED%==0 (
    echo No DLLs found for configuration "%CONFIG%" - build first.
    exit /b 1
)
exit /b 0

:copy_dir
if not exist "%~1\*.dll" goto :eof
for %%F in ("%~1\*.dll") do (
    copy /y "%%~fF" "%TARGET%\" >nul
    if errorlevel 1 (
        echo FAILED: %%~nxF
        set /a FAILED+=1
    ) else (
        echo copied: %%~nxF
        set /a COPIED+=1
    )
)
goto :eof
