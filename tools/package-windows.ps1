[CmdletBinding()]
param(
    [switch]$SkipBuild,
    [switch]$Force,
    [switch]$Release,
    [string]$MakeNsis,
    [string]$SignTool,
    [string]$SigningThumbprint,
    [string]$TimestampUrl = 'http://timestamp.digicert.com'
)

$ErrorActionPreference = 'Stop'
if (-not $IsWindows -and $env:OS -ne 'Windows_NT') {
    throw 'Windows packaging must run on Windows.'
}
if ($Release -and $env:RECENTRY_NATIVE_ACCEPTANCE -ne 'green') {
    throw 'Release packaging requires RECENTRY_NATIVE_ACCEPTANCE=green from the protected acceptance job.'
}
if ($Release -and ([string]::IsNullOrWhiteSpace($SignTool) -or [string]::IsNullOrWhiteSpace($SigningThumbprint))) {
    throw 'Release packaging requires -SignTool and -SigningThumbprint.'
}
if ([string]::IsNullOrWhiteSpace($SignTool) -xor [string]::IsNullOrWhiteSpace($SigningThumbprint)) {
    throw '-SignTool and -SigningThumbprint must be provided together.'
}
if (-not [string]::IsNullOrWhiteSpace($SignTool) -and -not (Test-Path -LiteralPath $SignTool -PathType Leaf)) {
    throw "signtool.exe was not found: $SignTool"
}

$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$targetRoot = [IO.Path]::GetFullPath((Join-Path $workspace 'target\package'))
$distRoot = [IO.Path]::GetFullPath((Join-Path $workspace 'dist'))
$metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }
$package = $metadata.packages | Where-Object name -eq 'recentry-host' | Select-Object -First 1
if ($null -eq $package) { throw 'recentry-host package metadata is missing.' }
$version = $package.version
if ($version -notmatch '^(\d+)\.(\d+)\.(\d+)(?:-[0-9A-Za-z.-]+)?$') {
    throw "Version cannot be represented by the Windows package: $version"
}
$major = [int]$Matches[1]
$minor = [int]$Matches[2]
$patch = [int]$Matches[3]
$revision = if ($version -match '-(?:[0-9A-Za-z-]+\.)*(\d+)$') { [int]$Matches[1] } else { 0 }
if (@($major, $minor, $patch, $revision) | Where-Object { $_ -gt 65535 }) {
    throw "Version component exceeds the Windows limit: $version"
}
$numericVersion = "$major.$minor.$patch.$revision"

$zipPath = Join-Path $distRoot "Recentry-$version-windows-x64.zip"
$installerPath = Join-Path $distRoot "Recentry-$version-windows-x64-setup.exe"
$checksumPath = Join-Path $distRoot "Recentry-$version-windows-x64-SHA256SUMS.txt"
$artifacts = @($zipPath, $installerPath, $checksumPath)
$existing = @($artifacts | Where-Object { Test-Path -LiteralPath $_ })
if ($existing.Count -gt 0 -and -not $Force) {
    throw "Release artifacts already exist. Re-run with -Force to replace only: $($existing -join ', ')"
}

if (-not $SkipBuild) {
    cargo build --workspace --release --locked
    if ($LASTEXITCODE -ne 0) { throw 'release build failed.' }
}

$releaseRoot = Join-Path $workspace 'target\release'
$hostBinary = Join-Path $releaseRoot 'recentry.exe'
$uiBinary = Join-Path $releaseRoot 'recentry-ui.exe'
foreach ($path in @($hostBinary, $uiBinary)) {
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Required release binary is missing: $path"
    }
}

if ([string]::IsNullOrWhiteSpace($MakeNsis)) {
    $command = Get-Command makensis.exe -ErrorAction SilentlyContinue
    if ($null -ne $command) { $MakeNsis = $command.Source }
}
if ([string]::IsNullOrWhiteSpace($MakeNsis) -and -not [string]::IsNullOrWhiteSpace($env:NSIS_HOME)) {
    $MakeNsis = Join-Path $env:NSIS_HOME 'makensis.exe'
}
if ([string]::IsNullOrWhiteSpace($MakeNsis)) {
    foreach ($candidate in @(
        (Join-Path ${env:ProgramFiles(x86)} 'NSIS\makensis.exe'),
        (Join-Path $env:ProgramFiles 'NSIS\makensis.exe')
    )) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) {
            $MakeNsis = $candidate
            break
        }
    }
}
if ([string]::IsNullOrWhiteSpace($MakeNsis) -or -not (Test-Path -LiteralPath $MakeNsis -PathType Leaf)) {
    throw 'makensis.exe was not found. Install NSIS or pass -MakeNsis <path>.'
}

$staging = [IO.Path]::GetFullPath((Join-Path $targetRoot "Recentry-$version-windows-x64"))
$targetPrefix = $targetRoot.TrimEnd('\') + '\'
if (-not $staging.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Unsafe staging path: $staging"
}
if (Test-Path -LiteralPath $staging) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
New-Item -ItemType Directory -Force $staging, $distRoot | Out-Null

Copy-Item -LiteralPath $hostBinary -Destination (Join-Path $staging 'recentry.exe')
Copy-Item -LiteralPath $uiBinary -Destination (Join-Path $staging 'recentry-ui.exe')
foreach ($document in @('README.md', 'CHANGELOG.md', 'LICENSE')) {
    Copy-Item -LiteralPath (Join-Path $workspace $document) -Destination (Join-Path $staging $document)
}

function Invoke-AuthenticodeSign([string]$path) {
    if ([string]::IsNullOrWhiteSpace($SignTool)) { return }
    & $SignTool sign /fd SHA256 /sha1 $SigningThumbprint /tr $TimestampUrl /td SHA256 $path
    if ($LASTEXITCODE -ne 0) { throw "Authenticode signing failed: $path" }
    & $SignTool verify /pa /all $path
    if ($LASTEXITCODE -ne 0) { throw "Authenticode verification failed: $path" }
}

Invoke-AuthenticodeSign (Join-Path $staging 'recentry.exe')
Invoke-AuthenticodeSign (Join-Path $staging 'recentry-ui.exe')

foreach ($path in $existing) { Remove-Item -LiteralPath $path -Force }
Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zipPath -CompressionLevel Optimal

$installerScript = Join-Path $workspace 'packaging\windows\installer.nsi'
& $MakeNsis '/V2' "/DVERSION=$version" "/DNUMERIC_VERSION=$numericVersion" "/DSOURCE_DIR=$staging" "/DOUTPUT_DIR=$distRoot" $installerScript
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $installerPath -PathType Leaf)) {
    throw 'NSIS packaging failed.'
}
Invoke-AuthenticodeSign $installerPath

$checksumLines = foreach ($path in @($installerPath, $zipPath)) {
    $hash = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
    "$hash  $([IO.Path]::GetFileName($path))"
}
[IO.File]::WriteAllLines($checksumPath, $checksumLines, [Text.UTF8Encoding]::new($false))

$mode = if ($Release) { 'release' } else { 'development' }
Write-Output "Created Recentry $version Windows x64 $mode artifacts:"
Get-Item -LiteralPath $installerPath, $zipPath, $checksumPath |
    Select-Object Name, Length, FullName
