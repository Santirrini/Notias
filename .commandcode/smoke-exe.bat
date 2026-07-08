@echo off
start "" "C:\Users\Jose Diaz\Documents\Notias\src-tauri\target\release\notias.exe"
timeout /t 6 /nobreak >nul
tasklist /FI "IMAGENAME eq notias.exe" /NH
taskkill /IM notias.exe /F >nul 2>&1
echo done