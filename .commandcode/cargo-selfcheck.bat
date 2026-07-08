@echo off
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" >nul
cd /d "C:\Users\Jose Diaz\Documents\Notias\src-tauri"
cargo run --example selfcheck --message-format=short 2>&1