# Windows build prerequisites

This guide explains how to set up a Windows machine from scratch so `pnpm tauri build`
produces a Windows installer. The toolchain is **MSVC + Rust stable + Node 20 + pnpm**.

## 1. Rust (MSVC toolchain)

Install [rustup](https://rustup.rs/) and let it pick the default host:

```powershell
# In PowerShell (admin not required)
Invoke-WebRequest -Uri https://win.rustup.rs/x86_64 -OutFile rustup-init.exe
.\rustup-init.exe -y
```

The default toolchain (`stable-x86_64-pc-windows-msvc`) is the one this project uses.
Verify:

```powershell
rustup show
# Default host: x86_64-pc-windows-msvc
rustup target list --installed
# expected: x86_64-pc-windows-msvc, x86_64-pc-windows-gnu (both fine to have)
```

> **Do not** put a custom `src-tauri/.cargo/config.toml` overriding the default target.
> This project historically had one forcing `x86_64-pc-windows-gnu` with a vendored
> LLVM-MinGW; that was the root cause of the "Windows SDK Lib missing" perception on
> this host. Without that override, the default MSVC toolchain is used.

## 2. Visual Studio Build Tools + Windows SDK

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022)
with the **"Desktop development with C++"** workload. This pulls in:

- MSVC compiler (`cl.exe`) and linker (`link.exe`)
- Windows SDK (ucrt + um headers and libs)

Notias was developed with these exact versions installed at the paths below; any
modern equivalent works:

| Component          | Path on this machine                                      |
|--------------------|-----------------------------------------------------------|
| MSVC v14.44        | `C:\BuildTools\VC\Tools\MSVC\14.44.35207\`                |
| Windows SDK 10.0   | `C:\Program Files (x86)\Windows Kits\10\`                 |
| `cl.exe` / `link.exe` | `C:\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\` |
| `vcvars64.bat`     | `C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat`           |

To confirm the install is complete from any shell:

```cmd
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
where cl
where link
echo %LIB%
echo %INCLUDE%
```

`%LIB%` must include `...\Windows Kits\10\lib\10.x.x.x\um\x64` (where `kernel32.lib` lives).

## 3. Node + pnpm

```powershell
winget install OpenJS.NodeJS.LTS
npm install -g pnpm
```

Verify:

```powershell
node --version   # v20+
pnpm --version
```

## 4. Build commands

From the repo root:

```powershell
# Frontend only
pnpm install
pnpm build

# Full Windows installer (MSI + NSIS)
pnpm tauri build
```

If your PowerShell session can't see MSVC, prefix the build with `cmd /c`:

```cmd
call "C:\BuildTools\VC\Auxiliary\Build\vcvars64.bat" && pnpm tauri build
```

This repo ships helper batch files in `.commandcode/` that do the same:

- `.commandcode\cargo-check.bat`  — `cargo check` with MSVC env
- `.commandcode\cargo-test.bat`   — `cargo test --lib` with MSVC env
- `.commandcode\cargo-selfcheck.bat` — `cargo run --example selfcheck`
- `.commandcode\tauri-build.bat`  — `pnpm tauri build` with MSVC env
- `.commandcode\smoke-exe.bat`    — runs the release `.exe` for 6 seconds then kills it

## 5. Artifacts

After `pnpm tauri build` finishes:

- `src-tauri\target\release\notias.exe` — standalone binary
- `src-tauri\target\release\bundle\msi\notias_0.1.0_x64_en-US.msi` — Windows Installer
- `src-tauri\target\release\bundle\nsis\notias_0.1.0_x64-setup.exe` — NSIS setup

Both installers embed WebView2 bootstrap so end-users don't need a separate runtime.

## 6. Verification status on this host (2026-07-06)

| Check                          | Result  | Notes                                            |
|--------------------------------|---------|--------------------------------------------------|
| `cargo check`                  | ✅ PASS | 0 errors, 3 warnings (unused imports/vars)       |
| `cargo test --lib`             | ✅ PASS | 62 passed, 0 failed, 1 ignored (keyring smoke)   |
| `cargo run --example selfcheck`| ✅ PASS | phases 3-6 invariants verified                   |
| `pnpm build`                   | ✅ PASS | SvelteKit static build                           |
| `pnpm tauri build`             | ✅ PASS | MSI + NSIS produced                              |
| Release `.exe` smoke launch    | ✅ PASS | WebView2 spawns, no crash                        |