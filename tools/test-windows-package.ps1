[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
if (-not $IsWindows -and $env:OS -ne 'Windows_NT') {
    throw 'Windows package smoke tests must run on Windows.'
}
if (Get-Process recentry, recentry-ui -ErrorAction SilentlyContinue) {
    throw 'A Recentry process is already running. Quit it before package smoke tests.'
}

$workspace = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$metadata = cargo metadata --locked --no-deps --format-version 1 | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed.' }
$version = ($metadata.packages | Where-Object name -eq 'recentry-host' | Select-Object -First 1).version
$distRoot = Join-Path $workspace 'dist'
$zipPath = Join-Path $distRoot "Recentry-$version-windows-x64.zip"
$installerPath = Join-Path $distRoot "Recentry-$version-windows-x64-setup.exe"
foreach ($artifact in @($zipPath, $installerPath)) {
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
        throw "Package artifact is missing: $artifact"
    }
}

$scratchRoot = [IO.Path]::GetFullPath((Join-Path $workspace 'scratch\package-smoke'))
$scratchPrefix = [IO.Path]::GetFullPath((Join-Path $workspace 'scratch')).TrimEnd('\') + '\'
if (-not $scratchRoot.StartsWith($scratchPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Unsafe smoke-test root: $scratchRoot"
}
if (Test-Path -LiteralPath $scratchRoot) {
    Remove-Item -LiteralPath $scratchRoot -Recurse -Force
}
New-Item -ItemType Directory -Force $scratchRoot | Out-Null

$portableRoot = Join-Path $scratchRoot 'portable'
$installRoot = Join-Path $scratchRoot 'installed'
$appDataRoot = Join-Path $scratchRoot 'appdata'
$configRoot = Join-Path $appDataRoot 'Recentry'
$configPath = Join-Path $configRoot 'config.json'
$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\Recentry'
$shortcutPath = Join-Path $env:APPDATA 'Microsoft\Windows\Start Menu\Programs\Recentry.lnk'

function Invoke-Process([string]$file, [string]$arguments, [hashtable]$environment = @{}) {
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $file
    $start.Arguments = $arguments
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    foreach ($entry in $environment.GetEnumerator()) {
        $start.Environment[$entry.Key] = $entry.Value
    }
    $process = [Diagnostics.Process]::Start($start)
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) {
        throw "$file $arguments exited with $($process.ExitCode)."
    }
}

function Wait-Condition([scriptblock]$condition, [int]$timeoutMilliseconds = 5000) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    while ($timer.ElapsedMilliseconds -lt $timeoutMilliseconds) {
        if (& $condition) { return $true }
        Start-Sleep -Milliseconds 20
    }
    return $false
}

function Test-Runtime([string]$root) {
    $hostExe = Join-Path $root 'recentry.exe'
    $uiExe = Join-Path $root 'recentry-ui.exe'
    foreach ($path in @($hostExe, $uiExe)) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf)) {
            throw "Runtime file is missing: $path"
        }
    }

    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $hostExe
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.Environment['APPDATA'] = $appDataRoot
    $hostProcess = [Diagnostics.Process]::Start($start)
    if (-not (Wait-Condition { -not $hostProcess.HasExited })) {
        throw 'Packaged host did not remain running.'
    }
    Invoke-Process $hostExe 'show'
    if (-not (Wait-Condition {
        @(Get-Process recentry-ui -ErrorAction SilentlyContinue |
            Where-Object Path -eq $uiExe).Count -eq 1
    })) {
        throw "Packaged UI did not start from $root"
    }
    Invoke-Process $hostExe 'quit'
    if (-not (Wait-Condition { $hostProcess.HasExited })) {
        throw 'Packaged host did not stop cleanly.'
    }
}

$succeeded = $false
try {
    New-Item -ItemType Directory -Force $configRoot | Out-Null
    $config = @{
        version = 1
        language = 'en'
        hotkey = @{ ctrl = $true; alt = $true; shift = $true; win = $false; key = 'R' }
        autostart = $false
        vscode_path_override = $null
        first_run_completed = $true
    } | ConvertTo-Json -Depth 4
    [IO.File]::WriteAllText($configPath, $config, [Text.UTF8Encoding]::new($false))

    Expand-Archive -LiteralPath $zipPath -DestinationPath $portableRoot
    Test-Runtime $portableRoot

    Invoke-Process $installerPath "/S /D=$installRoot"
    if (-not (Test-Path -LiteralPath $uninstallKey)) {
        throw 'Installer did not register its per-user uninstall entry.'
    }
    if (-not (Test-Path -LiteralPath $shortcutPath -PathType Leaf)) {
        throw 'Installer did not create the current-user Start Menu shortcut.'
    }
    Invoke-Process $installerPath "/S /D=$installRoot"
    Test-Runtime $installRoot

    $uninstaller = Join-Path $installRoot 'uninstall.exe'
    Invoke-Process $uninstaller '/S'
    if (-not (Wait-Condition { -not (Test-Path -LiteralPath $installRoot) })) {
        throw 'Uninstaller did not remove the installation directory.'
    }
    if (Test-Path -LiteralPath $uninstallKey) {
        throw 'Uninstaller left its uninstall registry entry behind.'
    }
    if (Test-Path -LiteralPath $shortcutPath) {
        throw 'Uninstaller left its Start Menu shortcut behind.'
    }

    $succeeded = $true
    Write-Output "Portable launch and silent install/upgrade/uninstall passed for Recentry $version."
}
finally {
    foreach ($process in Get-Process recentry-ui, recentry -ErrorAction SilentlyContinue) {
        if ($process.Path.StartsWith($scratchRoot, [StringComparison]::OrdinalIgnoreCase)) {
            Stop-Process -Id $process.Id -Force
        }
    }
    $uninstaller = Join-Path $installRoot 'uninstall.exe'
    if (Test-Path -LiteralPath $uninstaller -PathType Leaf) {
        try { Invoke-Process $uninstaller '/S' } catch { Write-Warning $_ }
    }
    if ($succeeded -and (Test-Path -LiteralPath $scratchRoot)) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force
    }
}
