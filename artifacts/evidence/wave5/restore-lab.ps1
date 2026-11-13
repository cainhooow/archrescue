$ErrorActionPreference = 'Stop'
$carrier = Split-Path -Parent $MyInvocation.MyCommand.Path
$evidence = (Get-Volume -FileSystemLabel EVIDENCE).DriveLetter + ':\'
$backup = (Get-Volume -FileSystemLabel BACKUP).DriveLetter + ':\'
$agent = Join-Path $carrier RESTORE.EXE
$handoff = Join-Path $evidence first-boot-handoff.json
$operation = Get-Content $handoff -Raw | ConvertFrom-Json
$source = Join-Path $backup ('ArchRescue\archrescue-backup-' + $operation.restore.backup_operation)
$profile = & $agent perfil | ConvertFrom-Json
if ($LASTEXITCODE -ne 0 -or $profile.root -notlike '*\Users\RecoveryQA') { throw 'Unexpected destination account' }
$profile | ConvertTo-Json -Depth 5 | Set-Content (Join-Path $evidence installed-profile.json) -Encoding UTF8
$os = Get-CimInstance Win32_OperatingSystem
$disk = Get-Partition -DriveLetter $env:SystemDrive.TrimEnd(':') | Get-Disk
Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class Wave5Firmware {
    [DllImport("kernel32.dll", SetLastError=true)]
    static extern bool GetFirmwareType(out uint mode);
    public static uint Mode() {
        uint mode;
        if (!GetFirmwareType(out mode)) throw new System.ComponentModel.Win32Exception();
        return mode;
    }
}
'@
$boot = [ordered]@{
    hostname=$env:COMPUTERNAME
    username=$env:USERNAME
    sid=$profile.account_sid
    firmware=([Wave5Firmware]::Mode())
    caption=$os.Caption
    version=$os.Version
    build=$os.BuildNumber
    ubr=(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').UBR
    display_version=(Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion
    disk_guid=$disk.Guid
    disk_serial=$disk.SerialNumber
    explorer_running=[bool](Get-Process explorer -ErrorAction SilentlyContinue)
}
if ($boot.firmware -ne 2 -or $boot.disk_guid.Trim('{}') -ne $operation.target_disk_identity.identity.Substring(4) -or !$boot.explorer_running) {
    throw 'Installed Windows boot identity differs from the deployment record'
}
$boot | ConvertTo-Json | Set-Content (Join-Path $evidence installed-boot.json) -Encoding UTF8
foreach ($folder in 'Desktop','Documents','Downloads','Pictures') {
    if (!$profile.folders.$folder) { throw "Missing Known Folder $folder" }
    if (!(Test-Path -LiteralPath $profile.folders.$folder -PathType Container)) {
        New-Item -Path $profile.folders.$folder -ItemType Directory | Out-Null
    }
}
$existing = Join-Path $profile.folders.Desktop 'conflict.txt'
[IO.File]::WriteAllText($existing, "existing destination file")
$before = (Get-FileHash -Algorithm SHA256 $existing).Hash
$mapping = @('Documents','Documents','Desktop','Desktop','Downloads','Downloads','Pictures','Pictures')
foreach ($policy in 'abortar','ignorar','renomear') {
    $preview = & $agent previa-recuperacao $handoff $source $profile.account_sid $policy @mapping
    if ($LASTEXITCODE -ne 0) { throw "Could not preview $policy" }
    $plan = $preview | ConvertFrom-Json
    $preview | Set-Content (Join-Path $evidence ("restore-preview-" + $policy + ".json")) -Encoding UTF8
    if ($policy -eq 'ignorar') { continue }
    if ($policy -eq 'abortar' -and !$plan.blocked_by_conflicts) { throw 'Abort did not detect the existing file' }
    $authorization = 'AUTORIZAR-RESTAURACAO:' + $plan.fingerprint
    $ErrorActionPreference = 'Continue'
    $report = & $agent restaurar-recuperacao $handoff $source $profile.account_sid $policy $authorization @mapping
    $exit = $LASTEXITCODE
    $ErrorActionPreference = 'Stop'
    $report | Set-Content (Join-Path $evidence ("restore-result-" + $policy + ".json")) -Encoding UTF8
    if (($policy -eq 'abortar' -and $exit -eq 0) -or ($policy -eq 'renomear' -and $exit -ne 0)) {
        throw "Unexpected restore status for $policy (exit $exit)"
    }
}
$after = (Get-FileHash -Algorithm SHA256 $existing).Hash
if ($after -ne $before) { throw 'Conflict was overwritten' }
[ordered]@{before=$before; after=$after} | ConvertTo-Json | Set-Content (Join-Path $evidence installed-conflict.json) -Encoding UTF8
Write-Host 'ARCHRESCUE RESTORE DONE'
