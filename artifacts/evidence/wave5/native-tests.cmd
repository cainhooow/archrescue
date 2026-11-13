@echo off
setlocal
set T=%~d0
if "%~3"=="" exit /b 2
set ARCHRESCUE_TEST_SOURCE=%~1
set ARCHRESCUE_TEST_DESTINATION=%~2
if not exist "%~3" mkdir "%~3"
for %%A in ("%T%\TESTS\*.exe") do (
  echo ==== %%~nA ====
  "%%A" --test-threads=2 > "%~3\%%~nA.log" 2>&1
  if errorlevel 1 (echo FAILED > "%~3\%%~nA.exit") else (echo OK > "%~3\%%~nA.exit")
)
