# Rho Windows Build Environment

Date: 2026-07-16  
Validated release: `0.2.0-dev.2`
Repository root: `D:\Rho`

## Purpose

This document is the build and acceptance contract for the current Windows
prototype. An implementation agent should read it before changing desktop,
runtime, packaging or release code.

The current release build is intentionally Windows x64 only. It uses Tauri 2,
Rust, R, Ark and the existing `aisdk` R packages. It does not use Python,
Jupyter Server, JupyterLab or Electron.

## Authority And Non-Goals

- `scripts/build-windows-installer.ps1` is the authority for the release build
  target and machine-local tool paths.
- `runtime/dependencies.json` is the authority for supported R versions and
  the Ark version, release assets, sizes and checksums.
- `rho deps` and the `rho-runtime-deps` crate are the authority for dependency
  discovery, verification, shared caching and controlled project bindings.
- `scripts/bootstrap-ark-windows.ps1` is a compatibility wrapper around
  `rho deps ensure`; it does not download or expand Ark itself.
- `runtime/ark.json` is retained as a deprecated compatibility manifest only.
- `Cargo.lock` is the authority for Rust dependency versions.
- Do not commit the Ark executable. The dependency manager publishes it to its
  immutable cross-project cache and the installer build copies the verified
  entry into `desktop/resources/runtime` only for packaging.
- Do not modify or reinstall the `aisdk` family merely to make a build pass.
- Do not automatically install the newly built Rho over a version the user is
  currently running.

## Verified Machine Snapshot

These are the versions and paths used on the current development machine. The
paths are machine-specific; the versions and targets describe the environment
that produced the validated installer.

| Component | Verified value |
| --- | --- |
| OS | Windows x64, kernel `10.0.26200` |
| PowerShell | Windows PowerShell `5.1.26100.8875`, Desktop edition, 64-bit |
| Git | `2.49.0.windows.1` |
| R | `4.6.0` UCRT |
| Rscript shim | `E:\software-data\scoop\shims\rscript.exe` |
| R home | `E:\software-data\scoop\apps\r\current` |
| Node.js | `24.12.0` |
| npm / npx | `11.6.2` |
| Rust / Cargo | `1.97.0` |
| Release Rust host | `x86_64-pc-windows-gnu` |
| Rtools compiler | Rtools45 GCC `14.3.0` |
| Ark | `0.1.252`, Windows x64 |
| Tauri Rust crate | `2.11.5` |
| Tauri CLI | pinned `2.11.4` |
| WebView2Loader.dll | `1.0.3650.58`, x64 |
| aisdk | `1.5.0` |
| aisdk.console | `0.1.0` |

Current R library paths are:

```text
E:\software-data\RLibrary
E:\software-data\scoop\persist\r\site-library
E:\software-data\scoop\apps\r\4.6.0\library
```

The installer itself requires R 4.4 or later at runtime. R 4.6.0 is the
version used for current build and smoke-test evidence, not yet the declared
minimum runtime version.

## Machine-Local Build Paths

The installer script currently sets these paths explicitly:

```text
CARGO_HOME=E:\software-data\scoop\persist\rustup\.cargo
RUSTUP_HOME=E:\software-data\scoop\persist\rustup\.rustup
RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-gnu
Rtools bin=C:\rtools45\x86_64-w64-mingw32.static.posix\bin
```

Verify them before building:

```powershell
Test-Path E:\software-data\scoop\persist\rustup\.cargo
Test-Path E:\software-data\scoop\persist\rustup\.rustup
Test-Path C:\rtools45\x86_64-w64-mingw32.static.posix\bin
Test-Path D:\Rho\desktop\resources\WebView2Loader.dll
```

There are two Rust selections to distinguish:

- entering the repository normally activates
  `1.97.0-x86_64-pc-windows-msvc` through `rust-toolchain.toml`;
- the Windows installer script deliberately overrides this with
  `stable-x86_64-pc-windows-gnu` and places the Rtools GCC directory first on
  `PATH`.

The installer script is the release authority. An agent must not assume the
interactive `rustup` default is the packaging target.

## External And Cached Inputs

A clean first build may require network access for:

- the checksum-pinned official R installer when the user explicitly asks Rho
  to prepare it;
- the checksum-pinned Ark archive from the Posit Ark GitHub release;
- Rust crates referenced by `Cargo.lock`;
- checksum-resolved `@tauri-apps/cli@2.11.4` invoked through `npx`;
- Tauri's NSIS bundling tools if they are not already cached.

Ark is pinned exactly in `runtime/dependencies.json`:

```text
version: 0.1.252
sha256: a6c2c6ae931d0dd5e1f771243bf3df4f86968462bbd8e08ceeba7f2e53567e58
```

The Tauri CLI is pinned to `2.11.4`. For a build report, record the resolved
CLI version and do not silently update the script or `Cargo.lock` as part of
unrelated feature work.

## R And Ark Bootstrap

From a fresh PowerShell session:

```powershell
Set-Location D:\Rho
Rscript -e "cat(R.version.string, '\n'); cat(R.home(), '\n'); cat(paste(.libPaths(), collapse='\n'))"
powershell -ExecutionPolicy Bypass -File scripts\bootstrap-ark-windows.ps1
target\debug\rho.exe deps status --project D:\Rho --json
```

If no compatible R is present, the report stays `action_required`. An explicit
`rho deps ensure --project D:\Rho --install-r --json` prepares and verifies the
official R installer, but the user must approve and run that operating-system
installer before repeating bootstrap. The build script never elevates itself
or asks an Agent to install R silently.

The bootstrap script:

1. delegates discovery and preparation to `rho deps ensure`;
2. accepts only a compatible R discovered by Rho, including `RHO_RSCRIPT` when
   explicitly configured;
3. installs Ark into the shared Rho cache only after the pinned size and
   SHA-256 checks pass;
4. materializes the embedded `rho.bridge` package without requiring a separate
   R package prerequisite;
5. writes a controlled kernelspec under `.rho\runtime\bindings` with user,
   site and project startup files disabled;
6. verifies that the kernelspec references the Ark path from the dependency
   report and prints that kernelspec path as its only result.

The controlled startup is intentional. User or project `.Rprofile` files can
break a headless Ark session and must not be re-enabled during packaging work.

Use `rho deps cache-path --project D:\Rho --json` to resolve the machine-local
cache root. Expected managed files are:

```text
<rho-cache>\components\ark\0.1.252\windows-x64\ark.exe
<rho-cache>\components\ark\0.1.252\windows-x64\LICENSE
<rho-cache>\components\ark\0.1.252\windows-x64\NOTICE
<rho-cache>\components\ark\0.1.252\windows-x64\rho-install.json
.rho\runtime\bindings\r-<R-version>-ark-0.1.252-windows-x64\kernel.json
.rho\runtime\bindings\r-<R-version>-ark-0.1.252-windows-x64\runtime.json
```

## Validation Before Packaging

Run the narrow checks first, then the workspace suite:

```powershell
Set-Location D:\Rho
node --check desktop\dist\app.js
Rscript -e "testthat::test_local('r/rho.bridge')"
Rscript -e "testthat::test_local('r/rho.agent')"
cargo test --workspace
```

For release-target validation, repeat Rust tests with the same GNU environment
used by the packaging script:

```powershell
$env:CARGO_HOME = 'E:\software-data\scoop\persist\rustup\.cargo'
$env:RUSTUP_HOME = 'E:\software-data\scoop\persist\rustup\.rustup'
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
$env:PATH = 'C:\rtools45\x86_64-w64-mingw32.static.posix\bin;' + $env:CARGO_HOME + '\bin;' + $env:PATH
cargo test --workspace
```

The model credential and network are not required for Rust tests, bridge
tests, editor/Console use, plot generation or Environment inspection. They are
required only for a real Agent-model smoke test.

## Building The Installer

The canonical build command is:

```powershell
Set-Location D:\Rho
powershell -ExecutionPolicy Bypass -File scripts\build-windows-installer.ps1
```

The script performs these steps:

1. selects the GNU Rust toolchain and Rtools45 linker;
2. runs `rho deps ensure` and requires a ready Windows x64 report;
3. rejects Ark unless Rho marked it verified and its path is inside the Rho
   dependency cache;
4. copies `ark.exe`, `LICENSE`, `NOTICE` and the verification receipt into the
   Tauri resource tree, then re-hashes every copy;
5. runs `npx.cmd -y "@tauri-apps/cli@2.11.4" build` from
   `desktop\src-tauri`;
6. creates the per-user x64 NSIS installer.

Expected outputs:

```text
D:\Rho\target\release\rho-desktop.exe
D:\Rho\target\release\bundle\nsis\Rho_0.2.0-dev.2_x64-setup.exe
```

Validated `0.2.0-dev.2` artifact snapshot:

```text
Installer size: 15,572,612 bytes
Installer SHA-256: 2923E54F286492D44C9B494D1268A8004E489FE15B8A3D5181F7A6B66B5D8C05
rho-desktop.exe size: 38,139,476 bytes
```

The hash identifies the already validated artifact only. A legitimate rebuild
can have a different hash because timestamps and packaging metadata may
change.

## Package Contents And Runtime Requirements

The NSIS installer must contain or install:

- `rho-desktop.exe`;
- Ark `0.1.252`, its license and notice, and Rho's verification receipt;
- `WebView2Loader.dll` for the GNU Windows build;
- the embedded HTML, CSS and JavaScript frontend.

The target machine must provide:

- Windows 10 or Windows 11 x64;
- Microsoft Edge WebView2 Runtime;
- R 4.4 or later;
- `aisdk` plus a configured provider only when Agent turns are used.

Rho supports `RHO_RSCRIPT` for an explicit `Rscript.exe` when automatic R
discovery cannot find the intended installation.

The installer is currently unsigned. A SmartScreen unrecognized-publisher
warning is expected and is not itself a build failure.

## Smoke And Acceptance Checks

Development smoke checks:

```powershell
target\debug\rho-desktop.exe --smoke-test
target\debug\rho-desktop.exe --smoke-agent
```

`--smoke-test` must verify that the same persistent Workspace R can execute R
code, create a data frame, return a plot and expose the object in Environment.
`--smoke-agent` additionally requires a configured DeepSeek provider and must
exercise Agent R without creating a second scientific workspace.

For an installer acceptance run:

1. close any running Rho instance before installing;
2. install the generated NSIS package per user;
3. launch Rho from the Start Menu;
4. confirm Ark starts from `%LOCALAPPDATA%\Rho\resources\runtime\ark.exe`;
5. execute `x <- 1:5`, inspect `x` in Environment and create a plot;
6. resize each workbench divider and confirm the sizes persist after restart;
7. open, edit and save a project file without leaving the project root;
8. run a DeepSeek Agent turn only when credentials and network are available;
9. record the installer path, size, hash and all failed or skipped checks.

Do not overwrite an installed build while its Workspace R is active. Building
an installer does not require automatically installing it.

## Common Failures

### Ark runtime not found

Run `rho deps status --project D:\Rho --json`. If Ark is missing, run
`scripts\bootstrap-ark-windows.ps1`; if a cached entry fails verification, run
`rho deps repair --project D:\Rho --json`. Do not copy an arbitrary `ark.exe`
into `.rho` or the packaging resource directory.

### Wrong Rust linker or target

Check `RUSTUP_TOOLCHAIN`, `CARGO_HOME`, `RUSTUP_HOME` and the Rtools45 `PATH`
prefix. The packaging target must be GNU even though the repository's normal
directory override selects MSVC.

### R package or DLL load failure

Verify which `Rscript` is active, then inspect `R.home()` and `.libPaths()`.
Regenerate the Ark kernelspec after changing R installations or libraries.

### `npx` hangs or attempts a download

The script requests the Tauri CLI through npm. Confirm network policy and npm
cache state. Record the resolved CLI version. Do not replace Tauri or introduce
Electron as a workaround.

### WebView is blank or the executable does not start

Verify the Microsoft Edge WebView2 Runtime on the machine and ensure the x64
`WebView2Loader.dll` remains in the bundle resources.

### Agent smoke test fails while R execution works

Treat provider credentials, provider network access and `aisdk` configuration
as a separate runtime concern. A model-network failure does not invalidate the
non-Agent desktop build, but it must be reported as a skipped or failed Agent
acceptance gate.

## Required Build Report

An implementation agent handing work back for review must provide:

- commit or complete working-tree diff;
- `git status --short` output summary;
- exact Rust, R, Node, Tauri CLI and Ark versions used;
- commands run and exact test results;
- installer path, byte size and SHA-256;
- manual workflows tested and screenshots when UI behavior changed;
- skipped checks and the reason for each skip;
- known limitations;
- an explicit statement that no aisdk family repository was changed, or a
  separately approved explanation if it was.
