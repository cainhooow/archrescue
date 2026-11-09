# ArchRescue P0 · 0.0.1

Brazilian Windows recovery prototype by ArchGTi. Rust core, native Win32 pt-BR shell, local diagnostics and selective content backup. **P0 is not accepted: Windows executable linking, ISO generation and VM boot validation are blocked in the current Linux environment.** See [handoff](docs/P0-HANDOFF.md) and [acceptance matrix](docs/qa/P0-TEST-MATRIX.md).

The workspace initially contained only the roadmap. This implementation does not repair boot entries, format disks, deploy Windows, migrate accounts, bypass encryption, or send telemetry. Source licensing is awaiting an explicit owner decision; the open-source product intent is not a license grant.

## Portable build and tests

Observed toolchain: Rust/Cargo 1.99.1, Linux x86_64. Dependencies are locked in Cargo.lock. Run from the repository root:

```sh
cargo build --release --locked
./target/release/archrescue-boot-shell --help
./target/release/archrescue-boot-shell browse /absolute/fixture/directory
```

On Linux, Windows inventory explicitly returns unsupported; no drives are fabricated. Filesystem browsing, candidate detection, EFI/header evidence, reports and verified copy run on actual filesystem data. Linux filesystem identity is st_dev and is not proof of separate physical hardware. Windows uses volume GUIDs and disjoint physical extents. Unsupported identities block backup.

Tests require a deliberately selected disposable directory on a filesystem distinct from the workspace. They create only uniquely named fixture directories and remove only those directories. The executed Linux setup uses tmpfs for the destination; this is not persistent backup or a Windows virtual disk test.

```sh
mkdir -p /dev/shm/archrescue-tests/.agent-tmp
ARCHRESCUE_TEST_DESTINATION=/dev/shm/archrescue-tests/.agent-tmp ./scripts/test.sh
rustup target add x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings
```

## Windows / WinPE build

Install official Windows ADK Deployment Tools, matching WinPE add-on, Visual Studio x64 C++ build tools/Windows SDK and Rust MSVC target. Review your applicable licenses. Open an elevated tools environment with both ADK and MSVC commands on PATH:

```powershell
powershell -NoProfile -File packaging/winpe/build.ps1
# With a supporting ADK and firmware trust configuration:
# powershell -NoProfile -File packaging/winpe/build.ps1 -BootEx
```

The script records build/import logs and hashes under `artifacts/winpe-<id>/`; work WIMs stay in `.agent-tmp/`. It creates only an ISO, never a physical USB. `startnet.cmd` runs wpeinit with networking disabled, launches the native shell, records a RAM-disk startup log, and retains a console fallback. No extra PowerShell, WPF or WebView component is required inside WinPE. Runtime availability is unverified until [VM acceptance](tests/vm/ACCEPTANCE.md).

## Use

Launch `archrescue-boot-shell.exe` without arguments for the Windows GUI. The dark native shell contains Painel, Discos e Partições, Instalações Windows, Diagnóstico de Boot, Explorador de Arquivos, Backup and Relatórios. Enter an absolute source folder in Explorer, use Ctrl/Shift multi-selection, enter an existing destination folder on a separate disk, then confirm **Copiar seleção**. Cancellation retains partial files with an incomplete status. Export reports to an authorized persistent destination before reboot.

The diagnostic CLI uses absolute source/destination roots and relative selections:

```text
archrescue-boot-shell.exe inventory
archrescue-boot-shell.exe discover D:\
archrescue-boot-shell.exe boot E:\
archrescue-boot-shell.exe browse D:\Users\Fixture
archrescue-boot-shell.exe backup D:\Users\Fixture F:\Recovered Documents Pictures
archrescue-boot-shell.exe report F:\Reports D:\ E:\
```

CLI backup prints its complete structured result to stdout; preserve it in an authorized destination. CLI termination via Ctrl+C is not graceful cancellation; use the GUI cancel button for cooperative cancellation. CLI report collects a new diagnostic snapshot and does not automatically ingest previous CLI backup stdout. The GUI keeps its attempted backups in the session report.

Backup is content-only: ACLs, timestamps, alternate streams, EFS, reparse points, special files and sparse allocation are not preserved. Existing destination files are never overwritten. Sources must be offline and quiescent; this is not a hardened concurrent filesystem sandbox. Filesystem/hypervisor read-only attachment is recommended because the OS can update metadata even when application handles are read-only.

Version/build detection from offline registry contents, full BCD reference consistency, physical storage health, dynamic/RAID volumes, Windows runtime execution and GUI visual acceptance remain limitations. Do not use this unqualified P0 on irreplaceable data.
