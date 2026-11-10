# ArchRescue 0.0.1 P0 corrective homologation

Validation took place on November 9–10, 2026, São Paulo time. Guest logs may use UTC. The run identifier is `p0-corrective`. Raw command output, VM configurations, images, screenshots, helper source and independent readbacks are under `.agent-tmp/p0-corrective/`. The historical records for `64bf3a6` remain separate.

## A. Executive summary

Both historical native Windows test failures reproduced on an unchanged checkout. The corrected regressions now distinguish NTFS aliases from distinct case-colliding entries and verify a destination I/O failure after real copy progress. The final Windows release, portable test suite, clean WinPE build, two UEFI boot platforms, GUI recovery and Secure Boot have new run evidence. The QEMU recovery disk was enforced read-only, and a sacrificial file creation was denied in WinPE.

**ArchRescue P0 remains BLOCKED. Independent human review and recorded approval are still required.**

## B. Repository and environment

- Initial local and independently queried GitHub `main` were both `bfb1522f678a31989f020e9c9c962920433a001a`. The working tree was clean. The two existing local Codex branches were preserved. The historical application revision `64bf3a6c92702368a0cb4fe8dc66b9385536d3a4` differed from the initial HEAD only in documentation. Host checks matched the historical EXE and ISO hashes. See `baseline/host.log`.
- Local branch `codex/p0-corrective-homologation`. Source commits are `aa4d80fa4497e810c27340812088511b122bafdd` for the collision fixtures and `513c8de75fe6c1c6a6c2bc2e2221e32d3afe943c` for destination write-failure injection. The two source files changed are `crates/backup/src/lib.rs` and `crates/backup/src/tests.rs`. The evidence commit updates this report, the Windows evidence index and the VM acceptance procedure. The final exact documentation HEAD, tracked file list, clean working tree and remote-main check are in `final-audit/git-final.log`. Git remained local with no push or release publication.
- W10Val UUID `147958bc-a550-4bf1-af59-50981fec149e` was initially running. Batch SSH authenticated as `w10val\archrescue` through the existing loopback NAT forwarding. The new live checkpoint `ARCHRESCUE-P0-CORRECTIVE-PRETEST`, UUID `1452602a-8bec-4221-a152-302ffedad65a`, includes memory and the three attached disk chains. Restoring it would discard later guest writes. The original `ARCHRESCUE-PRE-TOOLCHAIN` checkpoint was preserved. The later KVM runs used a temporary save of the current W10Val state. W10Val was resumed after testing and the established SSH alias responded. See `baseline/w10val-resume.log`.
- Existing `ArchRescue-QA`, UUID `699389da-cde1-47b0-b018-536667e0cfee`, remained powered off with checkpoint `5dd9486d-965e-4b28-adfc-c1e0981dd97e`. The final run used a separate VirtualBox VM and a dedicated QEMU workspace. The two disposable disk images used in VirtualBox were full clones of the earlier fixtures.
- W10Val system disk was 50 GiB. Native fixture media were the 24 GiB source at SATA 2 with serial `VB23517dfa-6ebaf22d`, and the 4 GiB NTFS destination at SATA 3 with serial `VB5c7ed667-01a2871c`. The test runner checked the destination serial, capacity, role and filesystem before creating `T:\p0-corrective-rust`. Tests created source files only under the clean checkouts' ignored fixture directories. The physical host disks were outside the VM attachment scope.
- Initial host free space was 351 GiB, with 7.8 GiB available on the separate tmpfs. Native rustc/Cargo 1.99.1, MSVC 14.44.35207 and SDK 10.0.26100.0 were exercised with a compile, library, link and executable probe. ADK DISM 10.0.26100.9457 serviced WinPE 10.0.26100.1. Toolchains were reused. The host already had Linux Rust plus both x64 Windows targets. `Cargo.lock` matched on host and guest at SHA-256 `c8aa495d554a8c719ed990afced04af614da5ae66c1108fd3781fac905bb7874`.
- QEMU and OVMF were initially absent from PATH and the system firmware directories. QEMU 11.1.3, `qemu-img` and OVMF 202608-1 were run from local Arch package extracts. Package signatures were checked against the host's Arch package keyring. Host packages and virtualization services were left alone. KVM execution worked with W10Val temporarily saved and VirtualBox QA powered off. See `qemu-ovmf/packages.txt`, `package-signatures.log` and `launch-recovery.log`. [Arch QEMU package](https://archlinux.org/packages/extra/x86_64/qemu-system-x86/) · [Arch OVMF package](https://archlinux.org/packages/extra/any/edk2-ovmf/)
- A host reboot at 22:53 interrupted the first final CLI and ISO invocations. W10Val resumed from its current disks. The CLI was rerun in fresh fixture directories and the ISO task used a new build directory after checking that no WIM was mounted. Interrupted logs are retained alongside the successful runs. See `baseline/w10val-interruption.log`, `build/native-cli.log` and `winpe/interrupted.log`.

## C. Native Windows failure reproduction and correction

| Case | Unchanged baseline on W10Val | Correction and evidence | Verdict |
|---|---|---|---|
| `case_collisions_and_overlapping_selections_are_rejected` | Exit 101 in the full and isolated runs. At historical `tests.rs:353`, `recover(["tree"])` returned `Completed`. Writing `a` and then `A` through case-insensitive NTFS created one directory entry. The first assertion stopped the separate overlap check from running. | The NTFS fixture proves `create_new("A")` reports `AlreadyExists`, the directory contains one entry, and both names canonicalize to that entry. The pair of selected aliases must return `Conflict` with no output. On both systems, exclusive creation, entry count and distinct bytes prove that `k.txt` and U+212A coexist. They must conflict both as explicit selections and under directory traversal. Linux also proves distinct `a` and `A` entries. A one-entry tree independently tests parent/child overlap. | PASS |
| `unavailable_destination_never_completes` | Exit 101 in the full and isolated runs. At historical `tests.rs:300` the test callback attempted to rename the destination directory while its file handle remained open. Windows returned error 5, and the fixture's unwrap panicked. The backup never returned a result. | Public recovery still uses `File::write`. A private writer argument allows the test to let the normal copy write 65,536 real bytes, then return a deterministic I/O error at the next write. The existing error mapping, byte counters, state machine and verification gate run as usual. The report payload must show `Failed`, 65,536 copied out of 200,006 planned bytes, one unverified file with no final hash, and no second file. The test rereads the partial bytes and confirms the source hash and unrelated destination content. | PASS |

The application keys intended portable output paths with Rust Unicode lowercase conversion. This is a locale-independent comparison for these fixtures. It does not implement general Unicode normalization or assume that Windows' filename comparison matches Rust's conversion. Preflight rejects a conflicting key and existing destination entries. The executor still creates files exclusively with `create_new`, revalidates storage identity, and only reports completion after SHA-256 verification. The existing no-overwrite regression passed on both systems. There was no case-sensitive NTFS directory in this run.

The full baseline output and two exact test logs are in `native-tests/baseline-full.log` and `native-tests/baseline-exact-*.log`. Final tests executed from a clean Git-bundle clone after the bundle, revision and changed-file hashes were checked. See `build/guest-checkout.log` and `build/native-tests.log`.

## D. Rust and native Windows release regressions

| Execution | Passed | Failed | Ignored | Unexecuted |
|---|---:|---:|---:|---:|
| Unchanged W10Val workspace, x64 MSVC, `--no-fail-fast` | 26 | 2 | 0 | 0 eligible |
| Final W10Val workspace, x64 MSVC, `--no-fail-fast` | 30 | 0 | 0 | 0 eligible (4 Unix-only tests excluded at compile time) |
| Final Linux workspace, `scripts/test.sh` with verified tmpfs destination | 33 | 0 | 0 | 0 eligible (1 Windows-only NTFS test excluded at compile time) |

Rustfmt, x64 MSVC all-target check and Clippy with warnings denied passed on W10Val. Linux executed `scripts/test.sh` from the committed source with the disposable tmpfs destination on device 34 and the checkout on device 31. Linux-hosted Clippy passed for both installed Windows targets. See `build/native-build.log`, `native-tests/linux-committed-rerun.log` and `native-tests/linux-cross-*.log`.

The final Windows CLI rerun recorded 17 passes and exit 0. It covered build/version, help, disk inventory, GPT/EFI, NTFS identities and free space, offline Windows, EFI/BCD, browsing, three-file selective backup and independent SHA-256, report export, same volume, same physical disk, overlap, existing-file conflict, missing drive, ACL denial and unchanged source content. See `build/native-cli-rerun.log` and `build/native-cli-rerun/`. Cancellation was exercised in the native Rust and actual GUI runs. The CLI exposes no interactive cancellation flag.

W10Val's interactive session 1 matched the real `ArchRescueP0` HWND to the final executable path and hash. The GUI copied and verified 262,173 bytes. It cancelled another copy after observing 12,320,768 bytes of progress and recorded an unverified 20,250,624-byte partial. PowerShell independently matched the copy and source hashes and confirmed the partial length and exported report. An initial foreground check failed, and the retried workflow could not read the exit status from the already-running process. Two fresh lifecycle processes independently closed with exit 0, and maximize changed content width from 848 to 1,684 pixels. See `gui/gui-evidence/`, `gui/native-integrity.log` and `gui/native-lifecycle.log`.

## E. Source and diagnostic integrity

Four layers were checked separately.

1. **Application**. Copy opens the source for reading and revalidates source and destination identities. Native and WinPE operations independently preserved the fixture payloads. BCD enumeration uses a temporary copy on a verified separate physical disk.
2. **OS**. Native W10Val BCD tests verified an isolated enumeration with `TEMP`/`TMP` on T: and signature-only fallback with them on S:, which shares the BCD's physical disk. Both scratch directories were empty after the command. The BCD SHA-256 remained `44f55abbc0f02da7b8bce7c0864a937630cc01a47aafbbb63a4065913573185e`. WinPE reported the unmounted EFI partition and did not mount it during diagnostics. WinPE's later Secure Boot registry query read the running recovery guest's state.
3. **Hypervisor**. QEMU used a regular, fully resolved 24 GiB VDI clone, a read-only VDI format node and a read-only file node, with no snapshot or writable overlay. The USB storage device exposed serial `ARQEMUSRCRO01`. WinPE DiskPart tied disk 1 and F: to GPT ID `FA5511AF-DAB8-41CF-AF6E-AD17DCA3CC7C` and reported `Current Read-only State: Yes` while the configurable disk attribute was `No`. A dedicated Win32 `CREATE_NEW` probe on `F:\archrescue-fixture\p0-write-denial-probe.txt` returned 19 (`ERROR_WRITE_PROTECT`). The independent 3 GiB destination was writable and appeared as C:. [QEMU block options](https://www.qemu.org/docs/master/system/qemu-manpage.html)
4. **Independent readback**. The protected source VDI and its historical origin both matched SHA-256 `5b2d9f769639069fff76a5e5a5ec6f43239ac50235854faa93415f2403315282` after the run. Host tools read the partition images as regular files and matched all source payload and BCD hashes. The sacrificial pathname was absent. VirtualBox's recovery clone used normal writable media. Its payload and BCD were checked separately, and VirtualBox provides no strict source write-protection evidence. Oracle's immutable mode redirects writes into a child image. [VirtualBox disk modes](https://docs.oracle.com/en/virtualization/virtualbox/7.2/user/storage.html)

See `integrity/native-bcd-and-iso-trust.log`, `integrity/source-payload-before.sha256`, `qemu-ovmf/query-named-block-nodes.json`, `disk-readonly-final.png`, `write-denial-final.png`, `source-image-postcheck.log` and `source-after.log`.

## F. QEMU and OVMF

QEMU used Q35, KVM, two vCPUs, 2,304 MiB, no NIC, OVMF CODE SHA-256 `2febd26c0b4cf95a636a941afa37d64a552723443ed9ac72f763f6840da98cb4` and private variable storage. The first launch had the source connected before optical boot and reached a `REGISTRY_ERROR` screen after the optical prompt was missed. The identity of the Windows image in that attempt remains unresolved. The retry booted with only the final optical ISO and the nonbootable writable destination exposed. The read-only source was attached after observing the ArchRescue window.

The first complete recovery used the actual pt-BR window at 1280×800. The follow-up boot used explicit VGA settings for 800×600, and the live-removal session ran at 1280×720. Keyboard input, offline x64 Windows discovery, GPT/EFI and NTFS inventory, application exit, source read protection and recovery were exercised. The source's offline version was explicitly unavailable. See `qemu-ovmf/optical-observed.png`, `reboot-vga-observed.png`, `keyboard-navigation.ppm`, `runner-current.png`, `launch-recovery.log` and `destination-failure/active.png`. An earlier resolution attempt using global VGA properties left the display blank. The retry used an explicit VGA device.

## G. VirtualBox

VirtualBox 7.2.22r175245 ran `ArchRescue-P0-Corrective-QA` (UUID `193fbc89-1a92-4a6b-93b3-9413c35e1143`), EFI64, two vCPUs, 2,304 MiB and no NIC. The tested VirtualBox EFI image SHA-256 was `e92f2b20f43b41f38522a9c9d0ddd3f29bb5dea46c8022acf5f5ecd5ffabf849`. The final ISO occupied SATA 0. The disposable destination was 3 GiB, medium UUID `e91cfde6-6871-4ea5-aaca-140f78f1baef` at SATA 2. The 24 GiB source clone `17aa7e09-dd8d-4bbd-b306-451958a7c62d` was hot-plugged at SATA 1 after optical boot. The nonbootable auxiliary optical disc at SATA 3 contained only the local automation harness.

The final build booted and displayed the GUI at 1280×720 and 800×600. The 1280×720 session exercised all seven sections and the full recovery. The 800×600 reboot checked keyboard navigation, fallback, persistence and additional negative cases. All corrective VM writes went to the new disposable clones. See `virtualbox/config-1280.log`, `config-800.log`, `WinPE-1280x720.log`, `WinPE-800x600.log` and `800-fallback-verified.png`.

## H. WinPE GUI walkthrough

The QA driver targeted the visible window class, queried its process image, sent real Win32 button/list messages, inspected confirmation and status controls, and captured the screen only while ArchRescue owned the foreground. Backup and report export ran through the GUI. Separate CLI inventory, discovery and boot queries served as machine-readable cross-checks. The driver and its commands are retained under `gui/tool/`. The first VirtualBox driver run found the window before it was visible and exited. The corrected driver waits for visibility and was run again from a freshly recognized auxiliary disc.

| Section | GUI action and observation on the two hypervisors |
|---|---|
| Dashboard | Observed consultation complete, two disks and one offline Windows candidate after fixture attachment. |
| Disks and Partitions | Opened via a real button and tested keyboard navigation. Viewed both disks, GPT partitions, EFI flag and the NTFS volume capacities, with X: as WinPE's RAM disk. |
| Windows Installations | Observed the offline x64 Windows candidate, with version unavailable. |
| Boot Diagnostics | Viewed expected missing paths and the unmounted EFI limitation. Native Windows separately exercised BCD enumeration. |
| File Explorer | Rejected a nonexistent Z: path, browsed the fixture, opened the nested folder through the list control, returned to its parent and selected three entries. |
| Backup | Confirmed source/destination in the modal prompt. Copied and verified 262,173 bytes. Rejected an existing output, a same-volume destination and overlapping trees. Cancelled a 512 MiB operation after nonzero progress. |
| Reports | Exported JSON and text to the destination from the actual window, then restored the copied data with an independent tool. The underlying console was observed after closing the UI, and the runner relaunched the application there. |

The host read both writable images independently. Copy and restore each preserved the empty directory and the three files, including the empty file, the 262,144-byte binary and `nested/çao.txt`. The final cancellation retained 19,529,728 unverified bytes in QEMU and 30,277,632 unverified bytes in VirtualBox, with exact on-disk lengths and expected partial content. The JSON report recorded three zero-byte preflight failures before the separate cancelled operation. See the platforms' `readback-pre-reboot*` directories and logs, which contain the actual GUI screenshots, confirmations and status text. Both rebooted guests listed the existing outputs, and independent post-reboot readbacks matched the original report hashes and verified the copied and restored data again. See both `readback-post-reboot.log` files and the extracted `guest-after-reboot.txt` files. A separate QEMU success run captured live GUI progress at 134,152,192 of 536,870,912 bytes, then a verified completion. Host readback independently matched the full 512 MiB copy to the source SHA-256. See `gui/live-progress-7.png`, `gui/large-current.png` and `gui/large-verified.log`.

The VirtualBox follow-up exercised two more GUI negatives. With 338,112,512 free bytes, selecting a 512 MiB file failed preflight for insufficient capacity. A separate sacrificial ACL fixture denied read-data access to the WinPE user and returned error 5. Both destinations remained empty. In QEMU a temporary WinPE drive-letter alias gave the read-only source's EFI volume a separate path. The actual GUI rejected F: to P: as sharing a physical disk and recorded zero written bytes. The alias was removed. The separate WinPE CLI controls rejected an absent destination and an invalid BCD signature and excluded an empty Windows directory. See `virtualbox/negative-verification.log` and `destination-failure/independent-checks.log`.

### Live destination removal

On November 10 the 3 GiB disposable copy `destination-failure/loss-dest.vdi` was attached through QEMU USB storage, serial `ARQEMULOSS01`, with a 4 MiB/s write throttle. DiskPart identified it as writable, non-boot disk 0, GPT ID `6709FFBA-AFFA-4701-948F-1C0A5F015833`, NTFS C:. The source remained the separate read-only USB fixture. Once actual guest progress was visible and the disposable backing file had grown from 80,740,352 to 270,532,608 bytes, QMP removed only `lossdest` and recorded `DEVICE_DELETED`.

The final WinPE CLI exited 1 with Win32 433 at `Write destination bytes`. The JSON result transitioned from Running to Failed, recorded 198,770,688 bytes accepted by Windows out of 536,870,912 planned, and left the file unverified with no final hash. Immediately after reconnect the guest directory entry showed zero bytes. Offline readback after clean guest shutdown recovered 176,291,840 bytes matching the zero-filled source prefix. The differing numbers measure Windows write acknowledgements and post-removal recoverable data. The report claimed zero verified files, and the unrelated pre-existing copy and report hashes still matched. The protected source image hash also matched. See `destination-failure/live-removal.log`, `before-detach.png`, `result.png`, `reconnected.png`, `export/loss.json` and `independent-checks.log`.

## I. Secure Boot

The dedicated VirtualBox EFI64 VM `ef28c6e0-fa24-4b92-a6f5-388e5df57e52` had only the final ISO attached. Its private variable store used the default Oracle VirtualBox PK, Microsoft's 2011 and 2023 KEKs and Microsoft's five default db certificates, including Microsoft Windows Production PCA 2011 and Windows UEFI CA 2023. The recorded dbx had one SHA-256 entry. PK, KEK, db and dbx were exported and parsed independently. The raw EFI loader's SHA-256 was `34347d4b7e1683f981d7b45ca660d32d9bf1cac3a20f5fdfeff6049f49a5384a`. W10Val reported a valid Microsoft Windows Authenticode signature issued by Microsoft Windows Production PCA 2011.

The final ISO was built with `BootEx=false`. The script's optional `-BootEx` passes `/bootex` to MakeWinPEMedia, which selects the Windows UEFI CA 2023 chain. The observed boot used the existing 2011 chain, matching the tested db. WinPE actually booted at 800×600, launched the final application and reported guest `UEFISecureBootEnabled=0x1`. The startup log showed the final source SHA and exit 0. The result is scoped to this particular db/dbx and ISO. See `secure-boot/trust-database.txt`, `config-running.log`, `current.png` and `secure-boot-state-and-build.png`. [Microsoft MakeWinPEMedia options](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/makewinpemedia-command-line-options?view=windows-11) · [Oracle firmware variable controls](https://docs.oracle.com/en/virtualization/virtualbox/7.2/user/vboxmanage.html)

## J. Final artifacts

| Item | Verified identity |
|---|---|
| Build source | `513c8de75fe6c1c6a6c2bc2e2221e32d3afe943c` |
| Windows x64 executable | `artifacts/windows/513c8de/winpe-ef8c728a4e704c73b329df3724eb73d8/archrescue-boot-shell.exe`, 769,536 bytes, SHA-256 `075ebdd0fa52ca6d8273e8530fc5bef388f3055c840c7be7174a588bb0dc6272` |
| Local WinPE ISO | `artifacts/windows/513c8de/winpe-ef8c728a4e704c73b329df3724eb73d8/archrescue-0.0.1-x64.iso`, 400,230,400 bytes, SHA-256 `c54cc71bbcb15655f4721c834f5d956d12530dd575e78cdf6c094c013d642036` |
| Injected boot WIM | `SOURCES/BOOT.WIM` in that ISO, SHA-256 `6322c85b0dba3b868e52be58e15f0e1d4ebe53f206fc1d5be440e911be8a2141` |
| PE imports | AMD64 PE32+, static CRT. Direct imports include KERNEL32, GDI32, USER32, NTDLL, SETUPAPI, bcryptprimitives and the synch API set. Runtime boot and recovery exercised the loader. |
| Build and WIM state | The existing `packaging/winpe/build.ps1` ran in a clean x64 MSVC/ADK checkout via an elevated interactive task. Exit 0. No mounted WIMs before or after the completed build. Guest and host artifact hashes agreed. |

See the local artifact directory for `build-info.json`, transcript, PE details and `SHA256SUMS.txt`. The full task launcher log is `winpe/winpe-task.log`. ISO and QA media are local and excluded from Git.

## K. Test audit and independent review

| Regression | What prevents a false pass |
|---|---|
| NTFS alias | Exclusive creation must return `AlreadyExists`, the entries must resolve to one canonical path, and the operation must return a preflight conflict with zero output. A rejection only at exclusive output creation would leave a first copy and fail the test. |
| Distinct ASCII and Unicode collisions | Exclusive fixture creation and directory counts prove coexistence. Unicode fixture bytes are read separately before testing. Each relevant selection must return a conflict and leave an empty destination. Disabling lowercase comparison in an isolated Linux source extract made both collision tests fail. |
| Overlap | The fixture contains one ordinary file and no colliding names. The expected validation diagnostic is specific to parent/child overlap. Removing that guard in the isolated source extract made the regression fail even though duplicate collection still rejected the operation. |
| Mid-copy write error | The ordinary recovery entry point supplies the real file writer. The fixture writes a real chunk through that writer before injecting an error into the same error branch that handles a failing destination write. It checks exact state transitions, progress, error, file bytes, verification and unrelated content. Initializing the partial as verified made the test fail in the isolated source extract. The separate VM test observed a real failure at the production write boundary. |

All three mutation controls exited 101 with the expected assertion failures. The candidate and final suites ran from intact code. The fixtures have no ignore markers, success shortcuts or mocked path comparisons. See `final-audit/mutation-exits.txt` and the three mutation logs.

An independent human reviewer must inspect the attached evidence and record their identity, date, reviewed artifact SHA-256 values, platforms, deviations and decision. The reviewer should rerun or directly observe the optical boot and real recovery workflow on both platforms, check the protected-source denial and post-test hashes, and inspect the separate Secure Boot state. The procedure is in [VM acceptance](../../tests/vm/ACCEPTANCE.md). No independent human approval has been recorded.

The acceptance scope covers these local disposable VM fixtures and the recorded firmware trust database. Surprise removal demonstrates why an acknowledged byte count must be read alongside the verified status. Host recovery after interruption, tool harness retries and filesystem readback differences are documented above.

## L. Final acceptance matrix

| Gate | Verdict | Evidence |
|---|---|---|
| Windows toolchain | PASS | Native compile, static library, link and execution probe. Rust, MSVC, SDK and ADK versions recorded. |
| Native Rust tests | PASS | 30 executed and passed on the clean committed W10Val checkout. |
| Linux Rust tests | PASS | 33 executed and passed on a verified separate tmpfs destination. |
| Windows CLI | PASS | 17 checks on the final EXE, independent hashes and exit 0. |
| Windows GUI | PASS | Real session 1 window, verified recovery, cancellation, export, maximize and exit/restart regression. |
| NTFS collision handling | PASS | Default NTFS alias and coexisting Unicode regressions, independent overlap, no overwrite. |
| Destination loss | PASS | Deterministic write error after 65,536 bytes and separate live USB loss with error 433, failed state and zero verified files. |
| BCD isolation | PASS | Native separate-device enumeration and shared-device signature fallback. Temporary workspaces empty and source BCD hash unchanged. |
| ISO build | PASS | Clean existing build pipeline, exit 0, exact EXE/ISO/WIM hashes and no mounted WIMs after completion. |
| VirtualBox UEFI | PASS | Final ISO at 1280×720 and 800×600, actual window and recovery. |
| QEMU/OVMF UEFI | PASS | Final ISO, genuine GUI and storage operations, tested at 800×600, 1280×720 and 1280×800. |
| Source write protection | PASS | QEMU enforced read-only source. DiskPart effective read-only state, Win32 error 19 and unchanged image and payloads. |
| Backup integrity | PASS | Independently matched source, copy and restored data, including empty and Unicode entries. |
| WinPE GUI backup | PASS | All seven controls and a real GUI-driven recovery on both platforms. |
| WinPE cancellation | PASS | Progress followed by Cancelled and exact independent unverified partial lengths on both platforms. |
| Report persistence | PASS | Guest reboot and independent hashes for both reports and payloads on both platforms. |
| Fallback console | PASS | Alt+F4, startup log exit 0 and GUI restart from the recovery console. |
| Secure Boot | PASS | Final ISO under recorded VirtualBox PK/KEK/db/dbx, WinPE confirmed Secure Boot enabled. |
| Human sign-off | BLOCKED | Requires an independent human walkthrough and recorded approval. |

```text
WINDOWS TOOLCHAIN: PASS
WINDOWS NATIVE TESTS: PASS
WINDOWS GUI: PASS
WINPE ISO BUILD: PASS
VIRTUALBOX UEFI BOOT: PASS
QEMU/OVMF UEFI BOOT: PASS
GUEST-VISIBLE SOURCE WRITE PROTECTION: PASS
WINPE GUI RECOVERY: PASS
BACKUP DATA INTEGRITY: PASS
SECURE BOOT: PASS
INDEPENDENT HUMAN SIGN-OFF: BLOCKED
ARCHRESCUE P0: BLOCKED
```

**TECHNICAL VALIDATION COMPLETE — HUMAN SIGN-OFF PENDING**

Confidence is high for the exercised binary, ISO, fixtures and trust configuration. The conclusion is based on independently matched artifacts and payloads, test and process results, guest observations, and the actual storage failure and write-denial evidence.
