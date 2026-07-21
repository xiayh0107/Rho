$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

if ($env:OS -ne "Windows_NT") {
    throw "This installer build supports Windows only."
}
if ([System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture -ne [System.Runtime.InteropServices.Architecture]::X64) {
    throw "The current Rho installer target is Windows x64."
}

$repo = (Resolve-Path -LiteralPath (Split-Path -Parent $PSScriptRoot)).Path
$cargoHome = "E:\software-data\scoop\persist\rustup\.cargo"
$rustupHome = "E:\software-data\scoop\persist\rustup\.rustup"
$rtoolsBin = "C:\rtools45\x86_64-w64-mingw32.static.posix\bin"

$env:CARGO_HOME = $cargoHome
$env:RUSTUP_HOME = $rustupHome
$env:RUSTUP_TOOLCHAIN = "stable-x86_64-pc-windows-gnu"
$env:PATH = "$rtoolsBin;$cargoHome\bin;$env:PATH"

function Invoke-RhoJson {
    param([string[]]$Arguments)

    $output = (& cargo @Arguments | Out-String)
    if ($LASTEXITCODE -ne 0) {
        throw "cargo $($Arguments -join ' ') failed with exit code $LASTEXITCODE. $output"
    }
    try {
        $envelope = $output | ConvertFrom-Json
    }
    catch {
        throw "rho returned invalid JSON. $output"
    }
    if (-not $envelope.ok) {
        throw "rho dependency command failed. $($envelope.error)"
    }
    return $envelope.data
}

# The dependency manager is the only authority that downloads, verifies and
# publishes Ark. Packaging consumes its report instead of maintaining a second
# archive/checksum implementation.
$ensureArguments = @(
    "run", "--quiet", "-p", "rho-cli", "--",
    "deps", "ensure", "--project", $repo, "--json"
)
$dependencyReport = Invoke-RhoJson -Arguments $ensureArguments
if (-not $dependencyReport.ready) {
    $issue = if ($dependencyReport.issue) { "$($dependencyReport.issue.code): $($dependencyReport.issue.message)" } else { "dependency report is not ready" }
    throw "Workspace runtime dependencies are not ready. $issue"
}
if ($dependencyReport.platform -ne "windows-x64") {
    throw "Expected a windows-x64 dependency report, got $($dependencyReport.platform)."
}

$arkComponent = @($dependencyReport.components) |
    Where-Object { $_.name -eq "ark" -and $_.status -eq "ready" } |
    Select-Object -First 1
if (-not $arkComponent -or -not $arkComponent.path) {
    throw "The ready dependency report did not include an Ark executable."
}
if (-not $arkComponent.verified) {
    throw "Refusing to package an Ark executable that was not verified by rho deps."
}
$arkSource = (Resolve-Path -LiteralPath ([string]$arkComponent.path)).Path

$cacheArguments = @(
    "run", "--quiet", "-p", "rho-cli", "--",
    "deps", "cache-path", "--project", $repo, "--json"
)
$cacheReport = Invoke-RhoJson -Arguments $cacheArguments
$pathTrimCharacters = [char[]]@(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
)
$cacheRoot = (Resolve-Path -LiteralPath ([string]$cacheReport.cache_path)).Path.TrimEnd($pathTrimCharacters)
$cachePrefix = $cacheRoot + [System.IO.Path]::DirectorySeparatorChar
if (-not $arkSource.StartsWith($cachePrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing to package Ark outside the Rho verified dependency cache: $arkSource"
}

$runtimeSource = Split-Path -Parent $arkSource
$runtimeDestination = Join-Path $repo "desktop\resources\runtime"
New-Item -ItemType Directory -Path $runtimeDestination -Force | Out-Null
foreach ($name in @("ark.exe", "LICENSE", "NOTICE", "rho-install.json")) {
    $source = Join-Path $runtimeSource $name
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) {
        throw "Verified Ark cache entry is missing $name at $source. Run rho deps repair."
    }
    $destination = Join-Path $runtimeDestination $name
    Copy-Item -LiteralPath $source -Destination $destination -Force
    if ((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash -ne (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash) {
        throw "Packaged Ark resource copy failed verification: $name"
    }
}

Push-Location (Join-Path $repo "desktop\src-tauri")
try {
    & npx.cmd -y "@tauri-apps/cli@2.11.4" build
    if ($LASTEXITCODE -ne 0) {
        throw "Tauri build failed with exit code $LASTEXITCODE."
    }
}
finally {
    Pop-Location
}

$installer = Join-Path $repo "target\release\bundle\nsis\Rho_0.2.0-dev.2_x64-setup.exe"
Write-Host "Rho installer: $installer"
