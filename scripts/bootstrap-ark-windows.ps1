param(
    # Retained for callers of the legacy script. Rho now owns the runtime root,
    # so a custom location is rejected instead of creating an unmanaged copy.
    [string]$RuntimeRoot,
    [string]$ProjectRoot = (Join-Path $PSScriptRoot ".."),
    [switch]$Offline
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version 2.0

if ($env:OS -ne "Windows_NT") {
    throw "This bootstrap script supports Windows only."
}

$repositoryRoot = (Resolve-Path -LiteralPath $ProjectRoot).Path
$pathTrimCharacters = [char[]]@(
    [System.IO.Path]::DirectorySeparatorChar,
    [System.IO.Path]::AltDirectorySeparatorChar
)
$managedRuntimeRoot = [System.IO.Path]::GetFullPath(
    (Join-Path $repositoryRoot ".rho\runtime")
).TrimEnd($pathTrimCharacters)
if ($RuntimeRoot) {
    $requestedRuntimeRoot = [System.IO.Path]::GetFullPath($RuntimeRoot).TrimEnd($pathTrimCharacters)
    if ($requestedRuntimeRoot -ne $managedRuntimeRoot) {
        throw "Custom RuntimeRoot is no longer supported. Rho manages project bindings under $managedRuntimeRoot."
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "Cargo is required to run the source-checkout rho dependency manager."
}

$ensureArguments = @(
    "run", "--quiet", "-p", "rho-cli", "--",
    "deps", "ensure", "--project", $repositoryRoot, "--json"
)
if ($Offline) {
    $ensureArguments += "--offline"
}
$ensureOutput = (& cargo @ensureArguments | Out-String)
if ($LASTEXITCODE -ne 0) {
    throw "rho deps ensure failed with exit code $LASTEXITCODE. $ensureOutput"
}
try {
    $ensureEnvelope = $ensureOutput | ConvertFrom-Json
}
catch {
    throw "rho deps ensure returned invalid JSON. $ensureOutput"
}
if (-not $ensureEnvelope.ok) {
    throw "rho deps ensure failed. $($ensureEnvelope.error)"
}

$report = $ensureEnvelope.data
if (-not $report.ready) {
    $issue = if ($report.issue) { "$($report.issue.code): $($report.issue.message)" } else { "dependency report is not ready" }
    throw "Rho could not prepare Workspace R. $issue"
}

$arkComponent = @($report.components) |
    Where-Object { $_.name -eq "ark" -and $_.status -eq "ready" } |
    Select-Object -First 1
if (-not $arkComponent -or -not $arkComponent.path) {
    throw "The ready dependency report did not include an Ark executable."
}
if (-not $arkComponent.verified) {
    throw "The compatibility bootstrap requires Ark verified by Rho; unset RHO_ARK and retry."
}
$arkPath = (Resolve-Path -LiteralPath ([string]$arkComponent.path)).Path

# Newer rho-cli versions report the controlled binding directly. The fallback
# keeps this compatibility wrapper usable with the first dependency-manager
# release, which only wrote the path to runtime.json.
$bindingComponent = @($report.components) |
    Where-Object { $_.name -eq "binding" -and $_.status -eq "ready" -and $_.path } |
    Select-Object -First 1
$kernelSpec = $null
if ($bindingComponent) {
    $kernelSpec = (Resolve-Path -LiteralPath ([string]$bindingComponent.path)).Path
}
else {
    $bindingsRoot = Join-Path $managedRuntimeRoot "bindings"
    if (Test-Path -LiteralPath $bindingsRoot -PathType Container) {
        $matchingBindings = @(
            Get-ChildItem -LiteralPath $bindingsRoot -Filter "runtime.json" -File -Recurse |
                ForEach-Object {
                    $metadataPath = $_.FullName
                    try {
                        $runtime = Get-Content -LiteralPath $metadataPath -Raw | ConvertFrom-Json
                        $candidateKernel = Join-Path $_.Directory.FullName "kernel.json"
                        if (
                            $runtime.ark -and
                            (Test-Path -LiteralPath $candidateKernel -PathType Leaf) -and
                            ([System.IO.Path]::GetFullPath([string]$runtime.ark) -eq [System.IO.Path]::GetFullPath($arkPath))
                        ) {
                            [PSCustomObject]@{
                                Path = $candidateKernel
                                GeneratedAt = [string]$runtime.generated_at
                            }
                        }
                    }
                    catch {
                        Write-Verbose "Ignoring invalid managed binding metadata at ${metadataPath}: $_"
                    }
                }
        )
        if ($matchingBindings.Count -gt 0) {
            $kernelSpec = ($matchingBindings | Sort-Object GeneratedAt -Descending | Select-Object -First 1).Path
            $kernelSpec = (Resolve-Path -LiteralPath $kernelSpec).Path
        }
    }
}
if (-not $kernelSpec) {
    throw "rho deps ensure completed without a controlled project kernelspec."
}

$resolvedBindingsRoot = (Resolve-Path -LiteralPath (Join-Path $managedRuntimeRoot "bindings")).Path.TrimEnd($pathTrimCharacters)
$bindingPrefix = $resolvedBindingsRoot + [System.IO.Path]::DirectorySeparatorChar
if (-not $kernelSpec.StartsWith($bindingPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Refusing an unmanaged kernelspec outside $resolvedBindingsRoot."
}

$kernel = Get-Content -LiteralPath $kernelSpec -Raw | ConvertFrom-Json
if (-not $kernel.argv -or $kernel.argv.Count -eq 0) {
    throw "Managed kernelspec has no Ark command: $kernelSpec"
}
$kernelArk = [System.IO.Path]::GetFullPath([string]$kernel.argv[0])
if ($kernelArk -ne [System.IO.Path]::GetFullPath($arkPath)) {
    throw "Managed kernelspec does not reference the verified Ark executable from the dependency report."
}

Write-Output $kernelSpec
