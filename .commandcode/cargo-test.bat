@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "C:\Users\Jose Diaz\Documents\Notias\src-tauri"
cargo test --lib --message-format=short 2>&1