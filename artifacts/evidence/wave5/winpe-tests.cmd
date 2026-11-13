@echo off
setlocal
set T=%~d0
set E=
for %%D in (C D E F G H I J K L M N O P Q R S T U V W Y Z) do if exist %%D:\first-boot-handoff.json set E=%%D:
if not defined E exit /b 2
if not exist C:\Windows\Panther\Unattend.xml exit /b 3
if not exist C:\WinPETestSource mkdir C:\WinPETestSource
if not exist "%E%\WinPETestDestination" mkdir "%E%\WinPETestDestination"
call "%T%\TESTS.CMD" C:\WinPETestSource "%E%\WinPETestDestination" "%E%\winpe-tests"
