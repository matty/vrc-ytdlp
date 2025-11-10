@echo off
setlocal enabledelayedexpansion

REM ===================================================================
REM Installation Script
REM Installs to VRChat Tools directory
REM ===================================================================

echo.
echo ========================================
echo Installation Script
echo ========================================
echo.

REM Check if running as administrator
net session >nul 2>&1
if %errorLevel% neq 0 (
    echo ERROR: This script must be run as Administrator.
    echo Please right-click on this file and select "Run as administrator"
    echo.
    pause
    exit /b 1
)

REM Define paths
set "SOURCE_EXE=%~dp0yt-dlp-proxy.exe"
set "VRCHAT_TOOLS_DIR=%LOCALAPPDATA%Low\VRChat\VRChat\Tools"
set "DESTINATION=%VRCHAT_TOOLS_DIR%\yt-dlp.exe"
set "SOURCE_CONFIG=%~dp0config.json"
set "DEST_CONFIG=%VRCHAT_TOOLS_DIR%\config.json"

echo Checking installation requirements...
echo.

REM Check if source executable exists
if not exist "%SOURCE_EXE%" (
    echo ERROR: Source executable not found at:
    echo %SOURCE_EXE%
    echo.
    echo Please build the project first using: cargo build --release
    echo.
    pause
    exit /b 1
)

echo Source executable found: %SOURCE_EXE%

REM Create VRChat Tools directory if it doesn't exist
if not exist "%VRCHAT_TOOLS_DIR%" (
    echo Creating VRChat Tools directory...
    mkdir "%VRCHAT_TOOLS_DIR%" 2>nul
    if !errorLevel! neq 0 (
        echo ERROR: Failed to create directory: %VRCHAT_TOOLS_DIR%
        echo Please ensure VRChat is installed or create the directory manually.
        echo.
        pause
        exit /b 1
    )
    echo Created directory: %VRCHAT_TOOLS_DIR%
) else (
    echo VRChat Tools directory exists: %VRCHAT_TOOLS_DIR%
)

REM Check if destination file already exists and backup if needed
if exist "%DESTINATION%" (
    echo.
    echo WARNING: Existing yt-dlp.exe found at destination.
    set /p "BACKUP_CHOICE=Do you want to backup the existing file? (Y/N): "
    if /i "!BACKUP_CHOICE!"=="Y" (
        set "BACKUP_FILE=%VRCHAT_TOOLS_DIR%\yt-dlp.exe.backup.%date:~-4,4%%date:~-10,2%%date:~-7,2%_%time:~0,2%%time:~3,2%%time:~6,2%"
        set "BACKUP_FILE=!BACKUP_FILE: =0!"
        copy "%DESTINATION%" "!BACKUP_FILE!" >nul
        if !errorLevel! equ 0 (
            echo Backup created: !BACKUP_FILE!
        ) else (
            echo [!] Warning: Failed to create backup
        )
    )
)

echo.
echo Installing...

REM Copy the executable
copy "%SOURCE_EXE%" "%DESTINATION%" >nul
if %errorLevel% neq 0 (
    echo ERROR: Failed to copy executable to destination.
    echo Source: %SOURCE_EXE%
    echo Destination: %DESTINATION%
    echo.
    pause
    exit /b 1
)

echo File copied successfully

REM Handle config.json
if exist "%SOURCE_CONFIG%" (
    echo.
    echo Installing config.json...
    if exist "%DEST_CONFIG%" (
        set /p "OVERWRITE_CFG=Existing config.json found. Overwrite it? (Y/N): "
        if /i "!OVERWRITE_CFG!"=="Y" (
            copy /Y "%SOURCE_CONFIG%" "%DEST_CONFIG%" >nul
            if !errorLevel! equ 0 (
                echo config.json overwritten at: %DEST_CONFIG%
            ) else (
                echo [!] Warning: Failed to overwrite config.json
            )
        ) else (
            echo Skipped overwriting existing config.json
        )
    ) else (
        copy "%SOURCE_CONFIG%" "%DEST_CONFIG%" >nul
        if !errorLevel! equ 0 (
            echo config.json copied to: %DEST_CONFIG%
        ) else (
            echo [!] Warning: Failed to copy config.json
        )
    )
) else (
    echo [i] No config.json found next to installer: %SOURCE_CONFIG%
)

echo.
echo ========================================
echo Installation completed successfully!
echo ========================================
echo.
echo Installation Details:
echo - Installed to: %DESTINATION%
echo.

echo.
echo Installation script completed.
rem Use start to open the directory; quotes handle spaces
start "VRChat Tools" "%VRCHAT_TOOLS_DIR%"
pause