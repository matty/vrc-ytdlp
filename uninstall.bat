@echo off
setlocal enabledelayedexpansion

REM ===================================================================
REM Uninstallation Script
REM Removes yt-dlp.exe from VRChat Tools directory
REM ===================================================================

echo.
echo ========================================
echo vrc-ytdlp Uninstallation Script
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
set "VRCHAT_TOOLS_DIR=%LOCALAPPDATA%Low\VRChat\VRChat\Tools"
set "TARGET_FILE=%VRCHAT_TOOLS_DIR%\yt-dlp.exe"

echo Checking for installation...
echo.

REM Check if the file exists
if not exist "%TARGET_FILE%" (
    echo No installation found at: %TARGET_FILE%
    echo Nothing to uninstall.
    echo.
    pause
    exit /b 0
)

echo Found installation: %TARGET_FILE%
echo.

REM Confirm uninstallation
set /p "CONFIRM=Are you sure you want to uninstall yt-dlp.exe? (Y/N): "
if /i not "%CONFIRM%"=="Y" (
    echo Uninstallation cancelled.
    echo.
    pause
    exit /b 0
)

echo.
echo Proceeding with uninstallation...

REM Delete the file
echo Removing file...
del "%TARGET_FILE%" >nul 2>&1
if %errorLevel% equ 0 (
    echo File removed successfully
) else (
    echo [!] ERROR: Failed to remove file
    echo You may need to remove it manually: %TARGET_FILE%
    pause
    exit /b 1
)

echo.
echo ========================================
echo Uninstallation completed successfully!
echo ========================================
echo.
echo The yt-dlp.exe has been removed from: %VRCHAT_TOOLS_DIR%
echo.
pause