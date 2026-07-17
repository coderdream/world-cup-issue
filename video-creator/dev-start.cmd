@echo off
chcp 65001 >nul
setlocal
set APP_EXE=%~dp0src-tauri\target\x86_64-pc-windows-gnu\release\video_creator.exe

for /f "tokens=2 delims=," %%P in ('tasklist /fo csv /nh /fi "imagename eq video_creator.exe" 2^>nul') do (
  taskkill /pid %%~P /f >nul 2>nul
)

powershell -NoProfile -ExecutionPolicy Bypass -Command "Get-CimInstance Win32_Process | Where-Object { ($_.Name -match 'node|pnpm|cargo|vite') -and ($_.CommandLine -like '*video-creator*') } | ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }"

if not exist "%APP_EXE%" (
  echo 未找到最新开发版可执行文件：
  echo %APP_EXE%
  pause
  exit /b 1
)

start "" "%APP_EXE%"
