# P0 feature acceptance matrix

Target acceptance concerns the **Windows/WinPE bootable product**. Portable subtests PASS separately. No row below grants acceptance based solely on type checking or Linux execution.

Commands:

- **T**: `ARCHRESCUE_TEST_DESTINATION=/dev/shm/archrescue-tests/.agent-tmp scripts/test.sh`
- **C**: `ARCHRESCUE_TEST_DESTINATION=/dev/shm/archrescue-tests/.agent-tmp python3 scripts/cli_acceptance.py`
- **W**: `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings`
- **V**: [Windows build and VM walkthrough](../../tests/vm/ACCEPTANCE.md)

| Capability | Target status | Expected behavior | Actually observed | Procedure and evidence | Remaining limit |
|---|---|---|---|---|---|
| P0-F01 Boot environment | BLOCKED | UEFI ISO boots, shell launches, version and fallback available | Build kit exists; MSVC link attempt cannot find link.exe; no ISO or VM | V; [boot evidence](../evidence/P0-BOOT-EVIDENCE.md) | Windows/ADK/firmware/two VM platforms unavailable |
| P0-F02 pt-BR GUI | BLOCKED | Dark graphical shell, seven sections, live states and navigation | Native Win32 source passes W; no GUI execution or screenshot | W/V; [Windows lint](../evidence/raw/windows-clippy.txt) | Layout, focus, accessibility and runtime behavior untested |
| P0-F03 Disk enumeration | BLOCKED | Actual Windows devices, identities, capacity, bus, errors | SetupAPI/query adapter type-checks; Linux explicitly reports unsupported | W/V; [build results](../evidence/P0-BUILD-RESULTS.md) | No actual Windows API execution |
| P0-F04 Partition/volume inspection | BLOCKED | GPT/MBR, EFI, filesystem, sizes, malformed metadata handling | Bounded parser tests decode controlled GPT/EFI bytes and reject truncation/bounds | T/W/V; [Rust gates](../evidence/raw/rust-gates.txt) | Driver/layout/NTFS integration requires Windows |
| P0-F05 Offline Windows detection | BLOCKED | Corroborated installation candidates, architecture, version when known, limitations | Real synthetic directory/PE/hive fixtures accepted/rejected correctly; version explicitly unknown | T/C/V; [Rust gates](../evidence/raw/rust-gates.txt) | No legitimate offline Windows image tested; hive content authentication/version parsing absent |
| P0-F06 Disk space | BLOCKED | Reliable total/used/available values and errors | Actual statvfs free-space preflight and inconsistent metadata rejection pass | T/V; [safety evidence](../evidence/P0-STORAGE-SAFETY.md) | GetDiskFreeSpaceExW integration unavailable |
| P0-F07 Read-only EFI/BCD | BLOCKED | Locate EFI/BCD, inspect supported configuration with evidence | Missing, invalid and permission-denied file tests pass; Windows bcdedit path implemented | T/W/V; [negative tests](P0-NEGATIVE-TESTS.md) | BCD runtime output, reference consistency and unmounted EFI access not verified; no automatic root-cause verdict |
| P0-F08 Offline explorer | BLOCKED | Real navigation/metadata, multi-selection, error and destination controls | CLI lists real fixture entries; GUI controls type-check | T/C/W/V; [CLI summary](../evidence/raw/cli-summary.json) | Native GUI flow and Windows permissions not exercised |
| P0-F09 Selective recovery | BLOCKED | Separate validated destination, conflicts, progress, cancellation, hierarchy, hashes | Three files/262174 bytes copied and independently verified/restored on distinct Linux filesystems; negative tests pass | T/C/V; [safety evidence](../evidence/P0-STORAGE-SAFETY.md) | Windows disk identities, persistent VM destination, reboot/power-loss and GUI flow unverified; content-only limits apply |
| P0-F10 Reports/logs | BLOCKED | Version/build, actual inventory/operations/errors, JSON/text persistent export | Local report roundtrip passes; GUI includes attempted backups; CLI emits backup results separately | T/C/V; [CLI summary](../evidence/raw/cli-summary.json) | Full WinPE report and persistence after reboot untested |

Executed sub-gates: formatting PASS; Linux clippy PASS; MSVC-target clippy/type checking PASS; 31 tests PASS; Linux release build PASS; independent CLI fixture acceptance PASS. Windows executable link, ISO, target runtime and both VM runs remain BLOCKED. There is no final-source test failure in the executed portable suite; this does not establish target functionality.
