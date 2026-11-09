# Build only local ISO media. This script never writes a USB or physical disk.
[CmdletBinding()]
param([switch]$BootEx)
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
$id = [Guid]::NewGuid().ToString('N')
$output = Join-Path $repo "artifacts/winpe-$id"
$work = Join-Path $repo ".agent-tmp/winpe-$id"
$mount = Join-Path $work 'mount'
New-Item -ItemType Directory -Path $output | Out-Null
$mounted = $false
Start-Transcript -Path (Join-Path $output 'build.log') | Out-Null
function Check-Exit([string]$Step) { if ($LASTEXITCODE -ne 0) { throw "$Step failed with exit code $LASTEXITCODE" } }
try {
    $env:ARCHRESCUE_BUILD_ID = (& git rev-parse HEAD)
    & cargo build --release --locked --target x86_64-pc-windows-msvc -p archrescue-boot-shell
    Check-Exit 'Rust build'
    $exe = Join-Path $repo 'target/x86_64-pc-windows-msvc/release/archrescue-boot-shell.exe'
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
    Write-Host "Build artifact created at $output. Boot, GUI, API-set resolution and Secure Boot remain NOT TESTED. Do not publish without rights review."
} finally {
    if ($mounted) {
        # Only this invocation's WIM mount is discarded; no global cleanup or disk operation.
        & dism.exe /Unmount-Image "/MountDir:$mount" /Discard
        if ($LASTEXITCODE -ne 0) { Write-Warning "Manual recovery needed for this mount: $mount" }
    }
    Stop-Transcript | Out-Null
}
