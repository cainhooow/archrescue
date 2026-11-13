$ErrorActionPreference='Stop'
$carrier=Split-Path -Parent $MyInvocation.MyCommand.Path
$evidence = (Get-Volume -FileSystemLabel EVIDENCE).DriveLetter + ':\'
$backup = (Get-Volume -FileSystemLabel BACKUP).DriveLetter + ':\'
$agent = Join-Path $carrier RESTORE.EXE
$handoff = Join-Path $evidence first-boot-handoff.json
$operation=Get-Content $handoff -Raw | ConvertFrom-Json
$source=Join-Path $backup ('ArchRescue\archrescue-backup-' + $operation.restore.backup_operation)
$profile=& $agent perfil | ConvertFrom-Json
$mapping=@('Documents','Documents','Desktop','Desktop','Downloads','Downloads','Pictures','Pictures')
$preview=& $agent previa-recuperacao $handoff $source $profile.account_sid ignorar @mapping
if ($LASTEXITCODE -ne 0) { throw 'Skip preview failed' }
$plan=$preview | ConvertFrom-Json
$preview | Set-Content (Join-Path $evidence restore-preview-ignorar-execution.json) -Encoding UTF8
$ErrorActionPreference='Continue'
$report=& $agent restaurar-recuperacao $handoff $source $profile.account_sid ignorar ('AUTORIZAR-RESTAURACAO:'+ $plan.fingerprint) @mapping
$exit=$LASTEXITCODE
$ErrorActionPreference='Stop'
$report | Set-Content (Join-Path $evidence restore-result-ignorar.json) -Encoding UTF8
if ($exit -ne 0) { throw "Skip execution failed ($exit)" }
