@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "C:\Users\Jose Diaz\Documents\Notias"
start /B "" cmd /c "pnpm tauri dev 2>&1 > C:\Users\Jose Diaz\Documents\Notias\.commandcode\dev-out.log"
echo Tauri dev started, PID=%errorlevel%
timeout /t 60 /nobreak >nul
tasklist /FI "IMAGENAME eq notias.exe" /NH
taskkill /IM notias.exe /F >nul 2>&1
echo done