@echo off
chcp 65001 >nul
title ArchRescue 0.0.1 - Console de diagnostico
set "AR_LOG=%SYSTEMROOT%\Temp\archrescue-startup.log"
echo ArchRescue 0.0.1 startup > "%AR_LOG%"
wpeinit -unattend:"%SYSTEMROOT%\System32\ArchRescue-unattend.xml" >> "%AR_LOG%" 2>&1
if errorlevel 1 echo AVISO: wpeinit falhou. Consulte %AR_LOG%
"%SYSTEMROOT%\System32\archrescue-boot-shell.exe" --version >> "%AR_LOG%" 2>&1
"%SYSTEMROOT%\System32\archrescue-boot-shell.exe" >> "%AR_LOG%" 2>&1
echo Application exit code: %ERRORLEVEL% >> "%AR_LOG%"
echo A interface foi encerrada. Console de recuperacao disponivel.
echo Log temporario: %AR_LOG%
echo Exporte o relatorio e os logs para um destino persistente autorizado.
echo Para reabrir: archrescue-boot-shell.exe
cmd /k
