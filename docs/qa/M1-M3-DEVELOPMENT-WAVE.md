# ARCHRESCUE — M1/M2/M3 DEVELOPMENT WAVE REPORT

Engineering run on 2026-11-10 in America/Sao_Paulo. All Windows and virtual-media work was local. The [environment hygiene report](../evidence/ENVIRONMENT-HYGIENE.md) records the intervening capacity incident.

## A. Starting baseline

| Item | Identity |
|---|---|
| Starting branch and commit | `main`, `4250fb6d8f5bee7a642d71b32c82e884dc6e5c3f` |
| Previously validated application | `513c8de75fe6c1c6a6c2bc2e2221e32d3afe943c` |
| Local development branch | `codex/m1-m3-recovery-advancement` |
| Source of promoted application and ISO | `ae04e7dedf2e4eee1eda9324a853ed368ca98f71` |
| Final implementation commit | `1b14db2d479bf24f433300a04f4e79a0e5f09bfc` followed by this reporting commit |

P0 already provided WinPE boot, Win32 UI, offline Windows discovery, disk and volume inventory, EFI/BCD inspection, file browsing, one-shot verified copy and export. Historical P0 technical evidence is retained in the [corrective homologation](P0-CORRECTIVE-HOMOLOGATION.md). The project owner accepts the P0 automated baseline for continued engineering in its documented laboratory scope. The first formal human review remains pending until the product reaches its evidence-backed 50% milestone. Distribution rights remain a separate open release gate.

Linux ran Rust 1.99.1. W10Val ran native x64 MSVC Rust 1.99.1, MSVC 19.44.35229 and Windows SDK 10.0.26100.0. The local ISO build used ADK DISM 10.0.26100.9457 and the matching WinPE add-on. The QEMU guest used OVMF, no network adapter, independent virtual source and destination devices, and the registered fixture for repair. After cleanup W10Val remained below the 15 GiB floor, so all subsequent guest validation used the already promoted ISO.

## B. Implemented capabilities

| Capability | Implementation and behavior |
|---|---|
| Persistent backup | [Transaction API](../../crates/backup/src/transaction.rs) and [journal](../../crates/backup/src/transaction/journal.rs). The destination holds an immutable plan, checksummed file checkpoints, staged partial content and published verified output. |
| Safe resume | Current storage identities and separation are checked again. Source content and published files are independently reread. An incomplete file restarts in staging. Publication rejects an existing target. |
| Guided recovery | [Native Win32 shell](../../apps/boot-shell/src/gui.rs) implements Detect, Diagnose, Protect Data, Repair, Verify and Report, with background work, actual progress, cancellation, checkpoint discovery, a technician view and session export. |
| Diagnostics | [Windows detection](../../crates/windows-detection/src/lib.rs), [structured diagnosis](../../crates/diagnostics/src/recovery.rs) and [read-only storage queries](../../crates/storage/src/windows.rs) add candidate classifications, PE resource and loader evidence, WinRE evidence and same-disk partition relationships. |
| Controlled repair | [Repair plan](../../crates/evidence/src/repair.rs) and [Windows lab adapter](../../crates/evidence/src/repair/windows.rs) gate an explicit BCDBoot operation on the registered disposable GPT disk, the exact ESP and Windows partition, independent preservation, health checks and per-plan authorization. |
| Reporting | [Report](../../crates/evidence/src/lib.rs) adds findings, plans, transaction results, user decisions and errors to the existing local JSON and text export. |

## C. Architecture and compatibility

The [transaction and repair contracts](../architecture/RECOVERY-TRANSACTIONS.md) describe schema versions, storage identities, the exclusive operation lock, flush and rename semantics, verification and rollback. The original backup CLI and API retain their layout and one-shot behavior. The guided interface uses the new transaction entry points. Schema 1 reports add optional fields. A backup manifest and repair plan each use their own versioned schema 1.

## D. Backup and resume trace

QEMU exposed the protected source VDI as read-only at the file and VDI block layers. The fixture contained an empty directory, an empty file, a Unicode filename, a 262,144-byte binary and a 512 MiB binary. The backup device was a separate writable GPT/NTFS virtual disk. The source GPT ID was `fa5511af-dab8-41cf-af6e-ad17dca3cc7c`; the destination GPT ID was `6709ffba-affa-4701-948f-1c0a5f015833`.

1. At 800×600 the operator opened the source in the GUI, selected the two fixture directories, reviewed the plan for four files and confirmed the independent destination.
2. The UI showed 25,624,605 acknowledged bytes of 537,133,085 planned and 262,173 verified bytes in three files. The harness terminated the GUI with exit code 73. The fourth file was still staged and was absent from the published payload.
3. The next process discovered `ar-18e6ed37bc652c9c-4f0-0` with a persisted Running phase and a valid manifest. WinPE shut down and booted again with the same destination.
4. At 1280×720 the GUI rediscovered the operation. After operator confirmation, it reread three retained files, restarted the interrupted file and recorded 537,133,085 verified bytes across all four files.
5. The GUI exported its session report. `backup-verify` and an independent `xcopy` restore into a third directory both exited 0. Offline `ntfscat` readback matched the following SHA-256 values for both payload and restore. The protected source VDI SHA-256 matched its pre-run value.

| File | Bytes | SHA-256 |
|---|---:|---|
| `archrescue-fixture/empty.txt` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `archrescue-fixture/nested/payload.bin` | 262,144 | `6e69354705c1f9cd28e7fe8f48e4be374955eaa1666600d2764e5503ef30062b` |
| `archrescue-fixture/nested/çao.txt` | 29 | `7a1acacd61e9e13d031e60fb5496ff4bfcaf784780ce0c13d3959af1034793b5` |
| `gui-cancellation-final/payload-512m.bin` | 536,870,912 | `9acca8e8c22201155389f65abbf6bc9723edc7384ead80503839f49dcc56d767` |

Linux independently killed a separate CLI process with SIGKILL, observed an 880,640-byte partial file, discovered and resumed the transaction, then independently checked source, payload and restore SHA-256.

## E. Recovery GUI

The first and second WinPE sessions exercised real selection, plan confirmation, progress, restart discovery, verification, the technician view and JSON/text export. The GUI remained usable while the backup ran. The screenshots and session export are local evidence under `artifacts/evidence/m1-m3/qa-results/`.

- [Backup plan at 800×600](../../artifacts/evidence/m1-m3/qa-results/preview-800.png)
- [Real interrupted progress at 800×600](../../artifacts/evidence/m1-m3/qa-results/interrupted-800.png)
- [Checkpoint selection after WinPE restart at 1280×720](../../artifacts/evidence/m1-m3/qa-results/resume-1280.png)
- [Technician verification at 1280×720](../../artifacts/evidence/m1-m3/qa-results/technician-verified-1280.png)
- [Report export at 1280×720](../../artifacts/evidence/m1-m3/qa-results/export-1280-host.png)

Two harness invocations returned 1. The first passed a trailing backslash in a quoted Windows argument and the corrected invocation returned 0. The second resumed successfully but its 40-second polling deadline expired before the final reread. The same GUI process then showed completion, exported the verified result, and the separate CLI and offline readback verified it. The checkpoint list displayed the last recorded Running phase during discovery. Technician mode was observed by clicking its control. VirtualBox Tab and Space moved focus and activated Diagnose. The Alt+T mnemonic did not activate Technician mode in the QEMU session. The wider keyboard matrix and a new WinPE cancellation walkthrough remain open.

## F. Diagnostics

The actual Windows 10 fixture exposed matching x64 kernel and `winload.efi` headers, kernel file resource `6.2.19041.2965`, `ReAgent.xml` and `Winre.wim`. The tool recorded the resource value as evidence and left edition and service state unresolved. It associated the installation with one same-disk ESP and flagged the lack of an accessible ESP path or resolved BCD reference. The same observations appear in the JSON report and the native GUI.

Real guest roots exercised a valid offline installation, an absent installation and a boot failure on the separate repair clone. Native and portable filesystem contract fixtures covered an incomplete Windows directory, malformed kernel and registry markers, an invalid BCD signature, missing boot files, an unavailable path and multiple installation candidates. Ambiguous ESP topology is classified by the implementation. A separate genuine multi-ESP Windows boot fixture was not run.

## G. Controlled UEFI repair

The original Windows test image remained protected. For repair, its verified VDI was converted once to a disposable sparse raw base. The lab assigned unique GPT identities and renamed its existing `EFI/Microsoft/Boot/BCD` to `BCD.BROKEN`. QEMU used a disposable QCOW2 overlay on the broken base. The boot-only guest produced Windows Recovery error `0xc0000098` before any repair.

| Registered item | Value |
|---|---|
| GPT disk | `c0f637a0-4134-434e-b444-25e0f8b90b09` |
| Windows partition | `02d7c8c7-4c48-4c91-a803-24c883072bc4` |
| EFI system partition | `65fbaf7a-1b02-40b8-99bb-1b1bc13a6590` |
| Independent Windows data backup | `ar-18e6ee2c28a6d564-5bc-0`, three files and 262,173 verified bytes |
| EFI backup | Failed before a transaction could be created |
| Rollback | Disposable QCOW2 overlay over the hash-verified broken base. The broken base and overlay were retired after extracting verified evidence. |

WinPE DiskPart identified the registered clone as disk 1 and its FAT32 ESP as partition 2. The assigned drive S: exposed `BCD.BROKEN` to the stock console. `mountvol` returned the expected GUID and `fsutil dirty query S:` returned clean. The application's safe canonicalization returned Win32 error 3 for S:, verbatim S: and the volume GUID path. Cold SATA attachment reproduced the failure after the earlier USB attachment. The Windows data backup independently verified with the Windows drive moved from F: to D: on the same stable partition. The separate EFI backup could not be verified, so the exact authorization token for the blocked SATA plan still exited 1 with an unsupported-operation error. A wrong token exited 1 with a permission error. BCDBoot did not execute. Independent offline readback found no replacement BCD and matched the original bytes of BCD.BROKEN. Post-repair checks and a repaired Windows boot remain pending.

Native W10Val dry-run against its separate GPT test fixture resolved the exact Windows and ESP partition IDs, previewed `bcdboot.exe S:\Windows /s R: /f UEFI /v`, and blocked execution outside the registered WinPE clone. A wrong ESP on the independent data disk returned a blocked plan with no command. These native checks used the same promoted EXE and left the W10Val system disk outside the target set. To recreate the lab, convert the protected source VDI to one sparse raw file, set the recorded GPT IDs on that copy, rename the BCD with the ESP at byte offset 16,777,216 and place a disposable QCOW2 overlay above it. The logged base capacity is 24 GiB logical. The original and modified GPT maps, protected-source hash, base hashes, exact launch arguments and compact automation sources are in the local QA archive.

[Pre-repair boot failure](../../artifacts/evidence/m1-m3/qa-results/repair-before-boot.png) · [SATA WinPE blocker](../../artifacts/evidence/m1-m3/qa-results/repair-sata-esp-blocked.png) · [Authorization guard](../../artifacts/evidence/m1-m3/qa-results/repair-command-blocked.png)

## H. Tests

| Layer | Observed result |
|---|---|
| Linux Rust | 38 passed, 0 failed, 0 ignored. `scripts/test.sh` used a separate disposable tmpfs and included rustfmt and Clippy with warnings denied. |
| Windows x64 MSVC Rust | 36 passed, 0 failed, 0 ignored on the clean transferred commit. Native formatting, check and Clippy passed. Linux cross-target Clippy with warnings denied passed separately. |
| CLI and release | Linux workspace release linked. The original CLI smoke verified one-shot copy, collision, export and independent restore. Linux process-kill resume passed. W10Val built and executed the promoted x64 EXE. |
| QEMU/OVMF WinPE | New ISO booted at 800×600 and 1280×720. Real GUI backup, forced interruption, restart, checkpoint discovery, resume, report export and independent backup and restore readback passed. Two harness invocations exited 1 as described in section E. |
| VirtualBox EFI | Same promoted ISO booted at 800×600 on the network-disconnected optical-only fixture. Tab and Space opened Diagnose. WinPE reported `UEFISecureBootEnabled=0x1` and the expected build ID. The previous optical medium was restored after shutdown. |
| Negative and fault tests | Current automated suite covers controlled mid-copy write error, cancellation, insufficient capacity, duplicate selections, same device/volume, conflicts, unsafe links, source changes, moved operations, corrupt checkpoints and altered readback. Real WinPE termination and drive-letter relocation were exercised. New physical disconnect/full-device scenarios and abrupt power-cut qualification remain open. |
| UEFI repair | Native dry-run and wrong-ESP rejection passed. Real QEMU pre-repair boot failure and WinPE wrong-token/blocked-plan controls passed. Write and repaired boot are blocked by the verified ESP preservation prerequisite. |

The raw test commands, Windows transfer hash, source commit, platform details, exact exits and test logs are under `artifacts/evidence/m1-m3/linux`, `artifacts/evidence/m1-m3/windows` and `artifacts/evidence/m1-m3/qa-results`.

## I. Artifacts

All artifacts remain local. The ISO was built with the existing `packaging/winpe/build.ps1`, static CRT, the fixture-specific registration and the stated source commit. W10Val and host SHA-256 values agreed. The build exited 0 and the final DISM inventory was empty.

| Artifact | Location and SHA-256 |
|---|---|
| Windows x64 EXE | [Executable](../../artifacts/windows/ae04e7d/winpe-6152a4e2d4c84c90a2887ac3da1f0209/archrescue-boot-shell.exe), `b37670b9f1fb7322f18573cbc7bad3d090c3982568548c372f6c791a49bbb1b2` |
| Local WinPE ISO | [ISO](../../artifacts/windows/ae04e7d/winpe-6152a4e2d4c84c90a2887ac3da1f0209/archrescue-0.0.1-x64.iso), `8ed38a07939f71f0e1b6714109594950d3c4fe1acd8a71216a4812e163a68930` |
| Injected WIM | `SOURCES/BOOT.WIM` in the ISO, `18a065fc5e3cf90c775a4d59d9037d4bca9ba18735987242e4e2beb51a4d5ac1` |
| Linux release EXE | External Cargo cache, `9b81035308cad01fe953f2d027fbec201394169545f4d3769a7098ef60b1926d` |

[Build metadata](../../artifacts/windows/ae04e7d/winpe-6152a4e2d4c84c90a2887ac3da1f0209/build-info.json) · [Build transcript](../../artifacts/windows/ae04e7d/winpe-6152a4e2d4c84c90a2887ac3da1f0209/build.log) · [VirtualBox Secure Boot and source identity](../../artifacts/evidence/m1-m3/qa-results/vbox-build-secureboot.png)

## J. ClickUp reconciliation

Initial inspection covered AR-00, P0 implementation, P0 automated QA, AR-01, AR-02, AR-03, AR-14, the first human review milestone and WinPE distribution rights. Follow-up reads identified the existing AR-02, AR-04, AR-05 and AR-14 subtasks. The updates compared each task's latest status and modification stamp with its inspection snapshot.

| Updated task | Final state |
|---|---|
| [AR-02 recovery interface](https://app.clickup.com/t/1dfqjx6uw4b), [guided wizard](https://app.clickup.com/t/1dfqjx6uw5f), [Technician mode](https://app.clickup.com/t/1dfqjx6uw5g) | In progress |
| [AR-03 inventory and diagnostics](https://app.clickup.com/t/1dfqjx6uw4c) | In progress |
| [AR-04 repair](https://app.clickup.com/t/1dfqjx6uw4d), [controlled BCD/EFI repair](https://app.clickup.com/t/1dfqjx6uw60) | In progress |
| [AR-05 backup](https://app.clickup.com/t/1dfqjx6uw4e), [resilient copy and resume](https://app.clickup.com/t/1dfqjx6uw6d) | In progress |
| [Backup manifest and verifiable hashes](https://app.clickup.com/t/1dfqjx6uw6e) | Complete |
| [AR-14 restart and power-loss qualification](https://app.clickup.com/t/1dfqjx6uw93) | In progress |

Eight ClickUp comments record the source commit, scope and evidence on the corresponding WinPE, wizard, diagnostics, repair, backup and QA tasks. The connector returned eight comment IDs. The initial automated P0 QA status remains Complete. The [50% human review milestone](https://app.clickup.com/t/1dfqjx6uyrk) and [WinPE redistribution-rights task](https://app.clickup.com/t/1dfqjx6uw8q) remain To Do. No new task or dependency was created.

## K. Remaining roadmap work

- Resolve the read-only ESP canonicalization failure in WinPE with a verified volume-path implementation. Recheck the full EFI preservation, wrong-target controls, exact authorization, BCDBoot execution, post-repair boot and rollback on a disposable clone.
- Complete the broader genuine Windows fixture matrix, including ambiguous ESP topology, locked or inaccessible volumes, device loss during resumable copy and abrupt power interruption.
- Exercise the new Cancel control in WinPE and complete the keyboard/focus matrix. Build the post-incident WinPE pipeline on W10Val when its C: volume meets the 15 GiB floor.
- Advance the guarded installation and profile restore work once the preservation and controlled repair gates are verified.

The release and distribution gates remain open. The [roadmap addendum](../../ARCHRESCUE_ROADMAP.md) describes capability-level progress without assigning a global percentage.

## L. Final assessment

```text
P0 AUTOMATED TECHNICAL BASELINE: PASS — documented P0 laboratory scope
PERSISTENT BACKUP MANIFEST: PASS
SAFE BACKUP RESUME: PASS — interrupted process, WinPE restart, independent readback
GUIDED RECOVERY WIZARD: PASS — observed WinPE diagnosis and backup workflow
ENHANCED WINDOWS DIAGNOSTICS: NOT TESTED — full native fixture matrix, with tested findings above
EFI/BCD REPAIR PLANNING: PASS — native target, command preview and wrong-ESP control
CONTROLLED LAB EFI/BCD REPAIR: BLOCKED — WinPE cannot canonicalize the clone ESP
NATIVE WINDOWS REGRESSION: PASS
WINPE INTEGRATION: PASS — promoted ISO and observed recovery workflow
SOURCE DATA INTEGRITY: PASS — protected VDI and independent payload hashes matched
CLICKUP SYNCHRONIZATION: PASS
PUBLIC DISTRIBUTION RIGHTS: BLOCKED — licensing gate remains open
OVERALL DEVELOPMENT WAVE: BLOCKED — verified EFI preservation and repaired boot remain pending
```
