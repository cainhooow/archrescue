# Native Windows tests

Both historical failures were reproduced and corrected. The final native suite recorded 30 passed and zero failed. The results below describe the earlier run. [Corrective homologation](../../qa/P0-CORRECTIVE-HOMOLOGATION.md#c-native-windows-failure-reproduction-and-correction)

Final CLI and Rust suite source `64bf3a6c92702368a0cb4fe8dc66b9385536d3a4`, Windows 10 Home 22H2 x64, elevated SSH for CLI and workspace tests. The release process and artifact identity are recorded in [Windows build results](WINDOWS-BUILD-RESULTS.md).

## Fixtures

| Role | Verified VirtualBox medium | Windows identity and layout |
|---|---|---|
| Development OS and checkout | System disk on SATA 0 | Disk 0, 50 GiB, C: NTFS. The Rust unit tests generate source files in the checkout's ignored test directory. |
| Disposable native source | `23517dfa-c48f-4a16-8ddc-3c932df2ba6e`, SATA 2 | Disk 1, serial `VB23517dfa-6ebaf22d`, 24 GiB GPT with a 260 MiB FAT32 EFI partition at R: and NTFS at S: |
| Disposable native destination | `5c7ed667-6b26-4926-8078-9de91c87a201`, SATA 3 | Disk 2, serial `VB5c7ed667-01a2871c`, 4 GiB GPT, NTFS T:. Rust test destination `T:\native-rust`. |

Each blank fixture disk was matched by medium UUID, SATA port, guest serial, byte capacity and non-boot/non-system status before initialization. The source offline Windows Home image came from the official Brazilian Portuguese x64 Windows 10 22H2 ISO, SHA-256 `0ca714572fcb5b8ce480acc3c93eaf2f34cc6507df5f693cff021bf4a32fac68`. DISM applied index 1 only to S:. BCDBoot populated the fixture EFI at R: using explicit `/s R: /f UEFI`. The offline image served as data and was never booted. [Microsoft ISO and published checksums](https://www.microsoft.com/pt-br/software-download/windows10iso)

## Release CLI

| Check | Result | Actual observation |
|---|---|---|
| `--version`, `--help` | PASS | Correct Windows x64 build identity and listed CLI commands |
| Disk enumeration | PASS | Three VirtualBox disks matched the known 50, 24 and 4 GiB capacities |
| GPT and EFI | PASS | Source and destination showed GPT. The source reported its 260 MiB EFI partition. |
| NTFS, volume identity and free space | PASS | Volume GUIDs, physical disk extents, total and free bytes agreed with independent Windows storage queries for S: and T: |
| Offline detection | PASS | The applied x64 candidate was identified. The running OS and empty Windows directory were excluded. Version was explicitly unavailable. |
| EFI and BCD | PASS | Existing EFI files and missing paths were distinguished. The BCD enumerated from an isolated copy. The source hash stayed unchanged. |
| File browsing | PASS | The nested directory, empty directory and empty file were listed with the expected types. |
| Selective copy and hashes | PASS | Three files completed with per-file verification. Independent PowerShell hashes matched source and destination, including the empty file and Unicode filename. |
| Report export | PASS | JSON and text existed on T:. The report contained the expected executable build, OS and architecture. |
| Access denied | PASS | A fixture ACL produced Win32 error 5, with a Permission error and no copied output. |
| Same volume and shared disk | PASS | Same-volume and separate-volume/same-disk destinations both exited 1 before writing. |
| Overlapping paths | PASS | Nested destination exited 1. A separate CLI control also rejected overlapping selections. |
| Existing file conflict | PASS | The destination conflict exited 1 and the existing file hash was unchanged. |
| Missing volume | PASS | Backup and browse failed cleanly on the verified absent drive letter. |
| Source content | PASS | Independent pre/post SHA-256 manifests matched for all three source payload files. |

The final CLI harness recorded 17 passing checks. The initial harness used the wrong JSON field for exclusion diagnostics and assumed locale-dependent listing order. Both assertions were corrected against actual JSON in the validation harness, with application behavior checked again on the final build.

## Native Rust suite and failure handling

`cargo test --workspace --locked --target x86_64-pc-windows-msvc --no-fail-fast` executed all native test binaries. Final outcome 26 passed and 2 failed. The Linux `scripts/test.sh` rerun passed all 31 tests, and Linux-hosted MSVC-target Clippy also passed. Test assertions were left intact.

| Test or capability | Result | Evidence |
|---|---|---|
| Cancellation before preflight | PASS | Native test produced Cancelled and an empty destination. |
| Cancellation after progress | PASS | Native test copied exactly 65,536 bytes and marked the partial file unverified. |
| Injected partial conflict and corruption | PASS | Native tests rejected completion after partial writes and detected readback mismatch. |
| `case_collisions_and_overlapping_selections_are_rejected` | FAIL | The default NTFS fixture treated `a` and `A` as one file. W10Val rejected per-directory case-sensitivity setup as unsupported. A supplemental CLI test created distinct `k.txt` and U+212A filenames and observed collision rejection, plus a separate overlap rejection. The original ASCII fixture remains blocked on this configuration. |
| `unavailable_destination_never_completes` | FAIL | The fault-injection callback panicked when Windows denied renaming a directory containing the open destination file. The in-flight unavailable-volume case remains blocked with this fixture. Preflight missing volume passed separately. |

## Source BCD isolation

On the initial build the application could not enumerate the source BCD. A controlled direct `bcdedit /store ... /enum all` on the disposable BCD changed its SHA-256 from `05543539bdb032d1b5ce64fcaef1fd11e7f9e53a18361a01eb5e569c89ea0a72` to `44f55abbc0f02da7b8bce7c0864a937630cc01a47aafbbb63a4065913573185e`. The direct command was a diagnostic control, performed before final source baselining.

Commit `9e624d2` stages a bounded copy on a verified separate physical disk and passes only that copy to BCDEdit. A same-physical-disk scratch test fell back to signature evidence and left no workspace on the source disk. Both final runs retained the original BCD hash at `44f55abbc0f02da7b8bce7c0864a937630cc01a47aafbbb63a4065913573185e`.

Raw evidence is local under `.agent-tmp/w10val/`. See `disks-preinit.log`, `disks-init.log`, `offline-fixture.log`, `bcd-path-diagnosis.log`, `native-selection-negatives.log`, `case-sensitive-fixture.log`, `final/bcd-regression.log`, `final/native-tests-full.log`, and `final/native-cli/`.

## Interactive GUI

The full control walkthrough used the BCD-fix build `9e624d2` on the W10Val desktop, session 1, PID 2664. The observed window class was `ArchRescueP0`, with title `ArchRescue 0.0.1 — Recuperação e diagnóstico`. The process path and SHA-256 matched the BCD-fix executable `fd9c8b0670f6d095332eaf1921186b50829edeed415a730038d32b78287ebaeb`. A temporary task used the existing interactive token and exercised the actual Win32 controls.

The walkthrough covered the pt-BR dashboard, disks, Windows candidates, diagnostics, the explorer, backup and reports. It exercised an inaccessible path, selected three entries, observed verified completion at 262,173 bytes, and independently hashed the copied files. A second operation reported progress at 20,971,520 of 536,870,912 bytes and was cancelled at 34,340,864 bytes. The file on T: matched that exact partial length and the report marked it unverified and Cancelled. JSON and text were exported to `T:\gui-reports-final`.

The first automation selected the companion console HWND. The successful pass bound the actual window class and HWND to PID 2664. Some checkpoint captures showed Explorer above the app, so those captures were excluded from visual evidence. The dashboard and final report were separately observed in the foreground. See local `final/w10val-gui-diagnostic.png`, `final/w10val-gui-report.png`, `final/gui-evidence/runtime.txt`, and `final/gui-verification.log`.

The final `64bf3a6` interactive lifecycle regression matched executable SHA-256 `2c292f8f55c6f4d2ec0358d39eba365a91472082e4c7b72c4d5f3a52f3753ca6`. The window's content width increased from 848 to 1,684 pixels on maximize. Two separate interactive processes, PIDs 4960 and 3824, opened and then exited 0 after the close message. The final CLI rerun again passed all 17 checks. The final Rust rerun reproduced the two fixture failures. The earlier GUI report also retained its hash after W10Val restarted. See `gui-fixed/gui-lifecycle.log`, `gui-fixed/native-cli.log`, `gui-fixed/native-tests-full.log`, and `w10val-restart-verification.log`.
