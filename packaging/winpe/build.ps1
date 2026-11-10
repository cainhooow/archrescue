# Build only local ISO media. This script never writes a USB or physical disk.
[CmdletBinding()]
param([switch]$BootEx, [switch]$KeepScratch)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT') { throw 'BLOCKED: Windows build host required.' }
$admin = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $admin.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) { throw 'BLOCKED: elevated ADK/Visual Studio tools environment required for DISM.' }
$repo = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
Set-Location $repo
foreach ($command in @('cargo','rustc','git','copype.cmd','MakeWinPEMedia.cmd','dism.exe','dumpbin.exe')) {
    if (-not (Get-Command $command -ErrorAction SilentlyContinue)) { throw "BLOCKED: missing $command. Load both ADK deployment tools and x64 MSVC tools." }
}
if (@(& git status --porcelain).Count -ne 0) { throw 'Commit the source before building an identifiable artifact.' }

function Require-FreeBytes([string]$Path, [long]$Required) {
    $drive = New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($Path))
    if ($drive.DriveType -ne [IO.DriveType]::Fixed -or $drive.AvailableFreeSpace -lt $Required) {
        throw "BLOCKED: $($drive.Name) has $($drive.AvailableFreeSpace) bytes available. This build requires $Required bytes."
    }
}

function Assert-PlainPath([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force
    while ($item) {
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Unexpected reparse point in build path: $($item.FullName)"
        }
        $item = $item.Parent
    }
}

function Measure-Scratch([string]$Path) {
    $pending = New-Object 'System.Collections.Generic.Stack[System.IO.FileSystemInfo]'
    $pending.Push((Get-Item -LiteralPath $Path -Force))
    [long]$bytes = 0
    while ($pending.Count) {
        $item = $pending.Pop()
        if ($item.Attributes -band [IO.FileAttributes]::ReparsePoint) {
            throw "Unexpected reparse point in scratch: $($item.FullName)"
        }
        if ($item -is [IO.DirectoryInfo]) {
            Get-ChildItem -LiteralPath $item.FullName -Force | ForEach-Object { $pending.Push($_) }
        } else {
            $bytes += $item.Length
        }
    }
    return $bytes
}

$scratchSetting = $env:ARCHRESCUE_WINPE_WORK_ROOT
if (-not $scratchSetting) { $scratchSetting = Join-Path $env:LOCALAPPDATA 'ArchRescue\scratch\winpe' }
if ($scratchSetting -notmatch '^[A-Za-z]:[\\/]') { throw 'WinPE scratch requires an absolute local drive path.' }
$scratchRoot = [IO.Path]::GetFullPath($scratchSetting)
if ($scratchRoot.StartsWith("$repo\", [StringComparison]::OrdinalIgnoreCase) -or $scratchRoot -eq $repo) {
    throw 'WinPE scratch must be outside the source checkout.'
}
$scratchBudget = 8GB
Require-FreeBytes "$env:SystemDrive\" 15GB
Require-FreeBytes $scratchRoot ($scratchBudget + 5GB)
Require-FreeBytes $repo 1GB
$existingAncestor = $scratchRoot
while (-not (Test-Path -LiteralPath $existingAncestor)) {
    $existingAncestor = Split-Path -Path $existingAncestor -Parent
}
Assert-PlainPath $existingAncestor
New-Item -ItemType Directory -Path $scratchRoot -Force | Out-Null
Assert-PlainPath $scratchRoot
$lock = [IO.File]::Open((Join-Path $scratchRoot 'build.lock'), 'OpenOrCreate', 'ReadWrite', 'None')

$id = [Guid]::NewGuid().ToString('N')
$output = Join-Path $repo "artifacts/winpe-$id"
$run = Join-Path $scratchRoot $id
$work = Join-Path $run 'staging'
$mount = Join-Path $work 'mount'
$cargoTarget = Join-Path $run 'cargo-target'
$previousCargoTarget = $env:CARGO_TARGET_DIR
New-Item -ItemType Directory -Path $run | Out-Null
$runIdentity = (Get-Item -LiteralPath $run).CreationTimeUtc
New-Item -ItemType Directory -Path $output | Out-Null
$cleanup = [ordered]@{
    id = $id; owner = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    scratch = $run; staging = $work; cargo_target = $cargoTarget
    created_utc = $runIdentity.ToString('o'); budget_bytes = $scratchBudget
    lifetime = 'Until ISO and evidence promotion or inspection of a failed run'
    dependencies = @($mount, $cargoTarget)
    retained_outputs = $output; state = 'active'
}
$cleanupPath = Join-Path $output 'scratch-manifest.json'
$cleanup | ConvertTo-Json | Set-Content -Encoding UTF8 $cleanupPath
$mounted = $false
$built = $false
Start-Transcript -Path (Join-Path $output 'build.log') | Out-Null
function Check-Exit([string]$Step) { if ($LASTEXITCODE -ne 0) { throw "$Step failed with exit code $LASTEXITCODE" } }
try {
    $env:CARGO_TARGET_DIR = $cargoTarget
    $env:ARCHRESCUE_BUILD_ID = (& git rev-parse HEAD)
    & cargo build --release --locked --target x86_64-pc-windows-msvc -p archrescue-boot-shell
    Check-Exit 'Rust build'
    $exe = Join-Path $cargoTarget 'x86_64-pc-windows-msvc/release/archrescue-boot-shell.exe'
    & dumpbin.exe /headers $exe | Out-File (Join-Path $output 'pe-headers.txt')
    Check-Exit 'PE header inspection'
    & dumpbin.exe /imports $exe | Out-File (Join-Path $output 'pe-imports.txt')
    Check-Exit 'PE import inspection'
    $dependencies = & dumpbin.exe /dependents $exe
    Check-Exit 'PE dependency inspection'
    $dependencies | Out-File (Join-Path $output 'pe-dependents.txt')
    if (($dependencies -join "`n") -match '(?i)(VCRUNTIME|MSVCP)\d+.*\.dll') { throw 'Unexpected dynamic Visual C++ runtime dependency; inspect static CRT configuration.' }
    & copype.cmd amd64 $work
    Check-Exit 'copype'
    & dism.exe /Mount-Image "/ImageFile:$work/media/sources/boot.wim" /Index:1 "/MountDir:$mount"
    Check-Exit 'Mount boot.wim'
    $mounted = $true
    $system32 = Join-Path $mount 'Windows/System32'
    $missing = @()
    $apiSets = @()
    foreach ($line in $dependencies) {
        if ($line -match '^\s+([a-zA-Z0-9_.-]+\.dll)\s*$') {
            $dll = $Matches[1]
            if ($dll -match '^(api|ext)-ms-') { $apiSets += $dll }
            elseif (-not (Test-Path (Join-Path $system32 $dll))) { $missing += $dll }
        }
    }
    if ($missing.Count -gt 0) { throw "Missing directly imported DLLs in WinPE: $($missing -join ', ')" }
    $apiSets | Out-File (Join-Path $output 'api-set-runtime-verification-required.txt')
    Copy-Item $exe (Join-Path $system32 'archrescue-boot-shell.exe')
    Copy-Item (Join-Path $PSScriptRoot 'startnet.cmd') (Join-Path $system32 'startnet.cmd')
    Copy-Item (Join-Path $PSScriptRoot 'unattend.xml') (Join-Path $system32 'ArchRescue-unattend.xml')
    $build = [ordered]@{
        schema_version = 1; version = '0.0.1'; target = 'x86_64-pc-windows-msvc'
        git_head = (& git rev-parse HEAD); rustc = (& rustc -V); cargo = (& cargo -V)
        executable_sha256 = (Get-FileHash -Algorithm SHA256 $exe).Hash
        lab_fixture = $env:ARCHRESCUE_LAB_FIXTURE
        dism_version = (Get-Item (Get-Command dism.exe).Source).VersionInfo.FileVersion
        bootex = [bool]$BootEx; boot_status = 'NOT TESTED'; secure_boot_status = 'NOT TESTED'
        api_set_resolution = 'Runtime execution required; file presence is insufficient'
    }
    $build | ConvertTo-Json | Set-Content -Encoding UTF8 (Join-Path $output 'build-info.json')
    Copy-Item (Join-Path $output 'build-info.json') (Join-Path $system32 'archrescue-build-info.json')
    & dism.exe /Unmount-Image "/MountDir:$mount" /Commit
    Check-Exit 'Commit local boot.wim'
    $mounted = $false
    $iso = Join-Path $output 'archrescue-0.0.1-x64.iso'
    if ($BootEx) { & MakeWinPEMedia.cmd /ISO $work $iso /bootex }
    else { & MakeWinPEMedia.cmd /ISO $work $iso }
    Check-Exit 'Create ISO'
    if (-not (Test-Path $iso) -or (Get-Item $iso).Length -eq 0) { throw 'ISO output is missing or empty.' }
    Copy-Item $exe (Join-Path $output 'archrescue-boot-shell.exe')
    Get-FileHash -Algorithm SHA256 $exe, $iso, (Join-Path $work 'media/sources/boot.wim') | Format-List | Out-File (Join-Path $output 'SHA256SUMS.txt')
    $built = $true
    Write-Host "Build artifact created at $output. Boot, GUI, API-set resolution and Secure Boot remain NOT TESTED. Do not publish without rights review."
} finally {
    if ($mounted) {
        # Only this invocation's WIM mount is discarded; no global cleanup or disk operation.
        & dism.exe /Unmount-Image "/MountDir:$mount" /Discard
        if ($LASTEXITCODE -ne 0) { Write-Warning "Manual recovery needed for this mount: $mount" }
    }
    try {
        $remainingMounts = & dism.exe /English /Get-MountedImageInfo
        Check-Exit 'Check WIM mount state'
        if ($remainingMounts -match [regex]::Escape($mount)) { throw "Scratch WIM is still mounted: $mount" }
        Assert-PlainPath $run
        if ((Get-Item -LiteralPath $run).CreationTimeUtc -ne $runIdentity) { throw 'Build scratch directory identity changed.' }

        $cleanup['logical_bytes_before_cleanup'] = Measure-Scratch $run
        if ($cleanup['logical_bytes_before_cleanup'] -gt $scratchBudget) {
            Write-Warning "WinPE scratch exceeded its $scratchBudget-byte budget."
        }
        if ($built -and -not $KeepScratch) {
            Remove-Item -LiteralPath $run -Recurse -Force
            if (Test-Path -LiteralPath $run) { throw "Scratch remains after deletion: $run" }
            $cleanup['state'] = 'cleaned'
            $cleanup['logical_bytes_remaining'] = 0
        } else {
            $cleanup['state'] = 'retained'
            $cleanup['logical_bytes_remaining'] = $cleanup['logical_bytes_before_cleanup']
            Write-Warning "WinPE scratch retained at $run. See $cleanupPath."
        }
    } catch {
        $cleanup['state'] = 'cleanup_blocked'
        $cleanup['cleanup_error'] = $_.ToString()
        Write-Warning $_
    } finally {
        $cleanup | ConvertTo-Json | Set-Content -Encoding UTF8 $cleanupPath
        $env:CARGO_TARGET_DIR = $previousCargoTarget
        $lock.Dispose()
        Stop-Transcript | Out-Null
    }
}
