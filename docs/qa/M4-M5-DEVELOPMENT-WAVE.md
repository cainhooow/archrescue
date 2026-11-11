# ARCHRESCUE — DEVELOPMENT WAVE 2 REPORT

## A. Starting state

| Item | Observation |
|---|---|
| Local repository | Clean `main` at `c1eba742e848d9ecfa79c4b879ddad16e0bcb773`, tracking `origin/main` |
| Working branch | `codex/m4-m5-deployment-restore` |
| Baseline | M1 native Win32 and WinPE boot, M2 read-only diagnosis and controlled repair planning, M3 verified checkpointed backup and WinPE restart/resume |
| Historical ISO | Built from `ae04e7dedf2e4eee1eda9324a853ed368ca98f71`. Used as the WinPE host for the Wave 2 executable on separate optical media |
| W10Val | Running at preflight. SSH returned `w10val\archrescue`. System disk and two fixture disks were attached via existing VirtualBox snapshots |
| W10Val free space | 12,470,177,792 bytes at preflight, about 11.6 GiB |
| Host free space | 309,535,924,224 bytes on the workspace filesystem. The Linux test destination was a disposable directory on separate tmpfs |

## B. Correction budget

**W10Val capacity.** Read-only preflight was recorded at 21:37 on November 10, 2026 (UTC−03:00). The focused follow-up ran from 21:55:45 to 21:55:46. One hypothesis was that known agent-owned build, ISO, and staging directories from prior runs could supply the missing space. The scoped inventory returned no qualifying directories. C: had 12,461,809,664 bytes free at the follow-up. No deletion or build was started on W10Val. The substantial-build floor remains 15 GiB. The host linked x64 Windows binaries against SDK/CRT libraries read from W10Val, then tested them in a disposable clone.

**EFI normalization.** The focused reproducer began at 22:05:10 on November 10. In WinPE 10.0.26100.1, the verified disposable GPT FAT32 ESP mounted as `S:\`. Rust canonicalization and `GetFinalPathNameByHandleW` with a DOS name returned Win32 error 3. GUID normalization returned the expected volume GUID, corroborated by `mountvol`. The first implementation used GUID normalization with extent and GPT checks. Runtime verification failed closed at `GetFileInformationByHandleEx(FileAttributeTagInfo)` with error 87. The second attempt used basic handle information to verify the directory and the reparse and encryption attribute bits, then retained the GUID and extent checks. Results and the exact window end are in section F. The run paused overnight. The second focused check ran from 07:40:24 to 07:45:05 on November 11. Timestamps record observation windows. An active-effort percentage was not instrumented. The two-hypothesis cap applied to EFI, and the capacity check stopped after the first hypothesis.

## C. M4 deliverables

- [Media inspection](../../crates/deployment/src/media.rs) accepts a direct WIM/ESD or a mounted media root containing `sources/install.wim` or `sources/install.esd`. It checks the header and required resource bounds, reads each index through inbox DISM, computes SHA-256 and checks an optional expected hash. It returns only observed metadata. Split media, ambiguous roots, and unmounted ISO files are outside the initial profile. The image hash identifies the bytes read. An independently supplied expected hash provides the comparison point for source integrity. [Microsoft DISM image options](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/dism/dism-image-management-command-line-options-s14)
- [Target qualification and planning](../../crates/deployment/src/plan.rs) uses the GPT disk ID and current storage inventory, volume access, topology, ESP, detected Windows installations and recovery files, firmware, capacity, device write protection, and predicted failure when available. It treats an unavailable health result as unknown and unresolved encryption or preservation on a populated disk as an intervention requirement. A supplied backup must be physically independent of the target and pass independent payload verification. A selective manifest supplies evidence for its own file set.
- The schema 1 JSON plan binds the media hash, index, target evidence, proposed partitions, capacity, risk, preservation, expected result and next action. Its SHA-256 is checked on export. The authorization contract binds confirmation to the fingerprint and reobserves the image and target. The planning profile is x64, UEFI and fixed GPT, with a 64 GiB minimum and a conservative image expansion reserve. Existing-installation repair and installation preserving data are explicitly unsupported in this deployment workflow. Installation execution remains unavailable.
- The [CLI](../../apps/boot-shell/src/deployment.rs) exposes `deployment-media`, `deployment-plan` and `deployment-authorize`. Use `inventory` for the disk ID. A blocked preview can be exported with reasons and returns a nonzero exit.

## D. M5 deliverables

The [Restore Agent CLI](../../apps/restore-agent/src/main.rs) runs in the destination Windows account. `perfil` resolves the token SID and Desktop, Documents, Downloads and Pictures through Windows known-folder APIs. `previa` maps explicit source directories from the backup to selected known folders under the account root. `restaurar` takes the same arguments and a confirmation bound to the reviewed plan. Account creation, ACL migration and arbitrary redirected locations are outside the current profile.

The [restore library](../../crates/restore/src/lib.rs) verifies the schema 1 manifest and recalculates payload SHA-256 without the old Windows installation. Profile and known-folder identities are checked before execution. Directory handles protect the path during a restore. Copying uses an exclusive temporary file in the same directory. The agent rereads and verifies the staged bytes, then publishes with a same-directory, no-replace rename through the open handle. It supports skip, abort and rename policies. The report records source and target relative paths, expected and actual hashes, bytes, publication, conflict handling, and errors for each file. [Microsoft FILE_RENAME_INFORMATION contract](https://learn.microsoft.com/windows-hardware/drivers/ddi/ntifs/ns-ntifs-_file_rename_information)

The disposable native demonstration created a real backup on a separate GPT/NTFS disk and restored two files under the distinct `ARWave2Restore` account, including a zero-byte file, a 3,600,123-byte Unicode file, and an empty directory. `Get-FileHash` independently matched source, backup and destination. The pre-existing destination file retained its original contents. Evidence and binary versions are listed in section G.

## E. GUI deliverables

The existing [Win32 recovery wizard](../../apps/boot-shell/src/gui/deployment.rs) adds **Implantar Windows** and **Restaurar arquivos**. It reads a real image on a worker, displays hash progress and extracted indices, exposes a GPT target and intent selector, shows a review with blockers, and exports a JSON plan. The restore section checks the payload and exports a reference for opening the separately runnable agent in installed Windows. It displays the available controls in pt-BR, with technician evidence in the existing mode. The 800×600 window displayed the WIM metadata and exported a clean-install plan from a real x64 Windows 10 WIM to a separate evidence volume. A real selective backup on independent NTFS storage produced a verified handoff reference. The complete keyboard-only walkthrough remains unverified. [Media](../../artifacts/evidence/m4-m5/winpe-gui-media.png), [plan](../../artifacts/evidence/m4-m5/winpe-gui-plan.png), [handoff](../../artifacts/evidence/m4-m5/winpe-gui-handoff.png)

## F. Safety validation

The runtime fixtures were dedicated VirtualBox media. The Windows clone boot disk was `20f37516-672c-41fe-abdf-647877995619`. Source and backup VDIs were `b9f5a943-f7b8-44bc-bedc-7fb4ee7d2f92` and `1b8135f3-c0a4-4f6c-b60b-61e248fbce72`. Before initialization, the harness checked the expected serials and sizes, RAW layout, and boot/system flags. WinPE used separate virtual disks for an empty 80 GiB GPT target and a 256 MiB EFI/export fixture. A local Windows installation ISO and the historical WinPE ISO were attached as optical media. The WinPE planner selected GPT disk `1dedf869-0728-4298-840b-088027c71e3c` with UEFI firmware, sufficient capacity and unknown device health. A later disposable partition change caused the exact earlier authorization to fail with `Evidências alteradas`. An invalid WIM was rejected before DISM. No deployment or boot repair command was executed against these disks. [Exported plan](../../artifacts/evidence/m4-m5/archrescue-deployment-c92cf5ea718ad87e3e419713d86540ff7ca47f90a00bff984989b70b37022cf4.json), [stale authorization result](../../artifacts/evidence/m4-m5/winpe-stale-plan.log)

The restore regressions cover corrupt or unsupported manifests, traversal even with recomputed checksums, missing or modified payload, wrong account, out-of-profile or unavailable paths, insufficient space, target directory replacement, a junction escape on Windows, late conflicts, partial writes, readback mismatch, and cancellation through the library. The original four native publication failures exposed a sharing violation from full-path publication while the parent was pinned. The initial standard-user run also hit access denied on a raw-device lookup during payload validation. The final adapter used same-directory handle-relative publication and a filesystem identity query without raw-device access. The final native suite exercised those paths.

The second EFI attempt opened the identified GPT FAT32 root through the verified GUID path. A subsequent targeted synthetic backup still failed before publication. `S:\EFI` and `S:\EFI\Microsoft\Boot\BCD.BROKEN` returned Win32 error 3 from DOS normalization, while GUID normalization resolved both on the same volume. That remaining nested-path failure keeps the verified EFI backup prerequisite unsatisfied. The next isolated repair task is to qualify component-by-component GUID normalization on the same GPT ESP, then repeat an independently verified backup before a disposable repaired-boot run. [First fix failure](../../artifacts/evidence/m4-m5/winpe-esp-final.err), [root result](../../artifacts/evidence/m4-m5/winpe-esp-second.json), [nested reproducer](../../artifacts/evidence/m4-m5/winpe-efi-nested.txt), [backup failure](../../artifacts/evidence/m4-m5/winpe-esp-backup.err)

## G. Tests and evidence

| Layer | Outcome |
|---|---|
| Linux | PASS. `scripts/test.sh` ran rustfmt check, Clippy with warnings denied and 59 tests, with zero failures or ignored tests. The destination was on separate tmpfs |
| Windows cross-target | PASS. x64 MSVC type checking and Clippy with warnings denied. Static-CRT host release link completed. The linker emitted LNK4099 for unavailable Microsoft library debug symbols |
| Windows native regression | PASS. 57 tests, zero failures or ignored tests, in the disposable Windows clone. An earlier run was FAIL with four restore failures and is retained |
| Windows Restore Agent end-to-end | PASS. Actual standard-user process, verified backup, known-folder mapping, rename on conflict, independent three-way hashes |
| Windows media and qualification | PASS. The final source reported six x64 Windows 10 indices, build 19041 and expanded sizes from the 4,853,776,048-byte WIM. PowerShell independently hashed the source. A native clean-install preview for the 768 MiB populated GPT fixture in BIOS reported Blocked and exited 1. ESD was not exercised |
| WinPE | PASS for WIM inventory, clean-install preview/export, rejection of invalid media and stale authorization, and verified restore handoff. Runtime executable was `17f65b9` on the historical WinPE environment. The final `9764c8b` change adds native locale-aware integer parsing and passed the native media and regression checks |
| EFI repair boot | NOT TESTED. No BCDBoot execution or repaired Windows boot was attempted |
| WinPE image build | BLOCKED by W10Val capacity. The local QA-only tools ISO holds the executable and probes and has no boot image |

Raw logs, reports, configuration and selected screenshots are in the [local evidence directory](../../artifacts/evidence/m4-m5). The [native hash transcript](../../artifacts/evidence/m4-m5/native-independent-final-source.log), [restore plan](../../artifacts/evidence/m4-m5/final-preview.json) and [restore report](../../artifacts/evidence/m4-m5/final-report.json) capture the native data comparison. The [first native regression log](../../artifacts/evidence/m4-m5/windows-native-tests.log) and [passing run](../../artifacts/evidence/m4-m5/windows-native-tests-final-source.log) preserve the initial failures and subsequent results.

## H. Environment state

| Measurement | Start | Final |
|---|---:|---:|
| Repository allocation | 31,364,481,024 bytes | 31,403,978,752 bytes |
| Existing repository scratch | 29,625,659,392 bytes | 29,625,659,392 bytes |
| External ArchRescue scratch and cache | 1,396,514,816 bytes | 3,168,301,056 bytes |
| Registered disposable run | 0 bytes before registration | 0 bytes after cleanup. 34,461,675,520 bytes immediately before cleanup |
| Host workspace filesystem available | 309,535,924,224 bytes | 304,360,820,736 bytes |
| W10Val C: available | 12,470,177,792 bytes | 12,461,809,664 bytes at the last measurement on November 10. VM powered off at final inspection |

The cleanup removed both disposable VMs, owned VDI and readback media, tool carriers, test transfer material, generated authentication material and the separate copy of SDK libraries. The host restart changed the Btrfs device number from 31 to 50. The path, owner, inode, original creation time and backing partition were checked before reconciling the generated manifest and running the repository cleanup command. W10Val retained its original snapshots and disk attachment UUIDs. The historical VM dependencies, installation media, reusable Cargo cache, selected Windows binaries, compact logs and screenshots remain available. [Closed scratch manifest](../../artifacts/evidence/m4-m5/scratch-manifest.json), [cleanup log](../../artifacts/evidence/m4-m5/scratch-cleanup.log), [final measurements](../../artifacts/evidence/m4-m5/final-storage.log)

## I. Git

| Commit | Scope |
|---|---|
| `526c533` | Deployment inspection, qualification and authorization |
| `c8a9e44` | Restore workspace crate and standalone executable |
| `1746c01` | Guided Win32 integration and restore handoff |
| `d9e9e23` | Native restore corrections, negative cases and EFI normalization |
| `4152cdd` | FAT32 basic handle attributes in the EFI fallback |
| `17f65b9` | Image version from indexed DISM metadata |
| `9764c8b` | Native locale grouping for DISM image sizes |

The branch starts at `c1eba742e848d9ecfa79c4b879ddad16e0bcb773`. The delivery executables identify source `9764c8b` in their `--version` output. [Host SHA-256 manifest](../../artifacts/windows/9764c8b/SHA256SUMS.txt), [native SHA-256 transcript](../../artifacts/evidence/m4-m5/native-locale-binary-hashes.log) The documentation commit is recorded as the final branch SHA in the delivery transcript. [Git delivery transcript](../../artifacts/evidence/m4-m5/git-delivery.txt) Changed source is under `apps/boot-shell`, `apps/restore-agent`, `crates/backup`, `crates/deployment`, `crates/restore`, `crates/storage`, and the workspace manifests. The branch and evidence are local. No push or release publication occurred.

## J. ClickUp

The first workspace search succeeded. Direct task reads then returned `RATE_LIMIT_EXCEEDED` at 100/100 daily calls. No remote task changes were made. The local roadmap records the partial delivery. On reconciliation, reread the current status and append evidence to the existing tasks. Keep the epic acceptance boxes open.

| Pending task | Proposed reconciliation |
|---|---|
| [AR-02 recovery interface](https://app.clickup.com/t/1dfqjx6uw4b) | In progress. Record deployment and restore handoff controls and runtime evidence |
| [AR-03 diagnostics](https://app.clickup.com/t/1dfqjx6uw4c) | Add the read-only storage evidence used by M4. Keep the broader fixture matrix open |
| [AR-04 repair](https://app.clickup.com/t/1dfqjx6uw4d) and [controlled EFI/BCD repair](https://app.clickup.com/t/1dfqjx6uw60) | In progress. Record the ESP reproduction and targeted result. Keep repaired boot qualification open |
| [AR-05 backup](https://app.clickup.com/t/1dfqjx6uw4e) | Record source-independent payload verification and the native backup/restore exercise |
| [AR-06 deployment](https://app.clickup.com/t/1dfqjx6uw4f) and [immutable plan](https://app.clickup.com/t/1dfqjx6uw6n) | In progress. Link M4 implementation and exported plan. Keep execution gates open |
| [AR-07 Restore Agent](https://app.clickup.com/t/1dfqjx6uw4g), [standalone agent](https://app.clickup.com/t/1dfqjx6uw6y) and [file conflicts](https://app.clickup.com/t/1dfqjx6uw74) | In progress. Link the executable, native report and independent hash check |
| [AR-14 QA](https://app.clickup.com/t/1dfqjx6uw4p) | Append the Linux, Windows and WinPE results, failed attempts, and W10Val capacity |
| [50% human review](https://app.clickup.com/t/1dfqjx6uyrk) and [WinPE distribution rights](https://app.clickup.com/t/1dfqjx6uw8q) | Preserve the review and redistribution gates. Neither status was changed |

## K. Next development wave

Implement one controlled Windows application path for a registered disposable UEFI/GPT target. Consume a freshly revalidated deployment plan and preserve independent rollback evidence. Boot the new Windows installation and run the Restore Agent under its new account. Extend supported media and storage qualification as required for that acceptance path.

## L. Final verdict

| Workstream | Outcome |
|---|---|
| M4 image inspection | DELIVERED for the validated WIM profile. ESD runtime NOT TESTED |
| M4 deployment planner | DELIVERED. Preview, export and stale-authorization refusal exercised |
| M4 target safety | DELIVERED for the initial x64 UEFI/GPT profile with conservative intervention gates |
| M5 Restore Agent | DELIVERED |
| M5 profile mapping | DELIVERED for the running Windows account and selected known folders |
| M5 real file restoration | DELIVERED. Source, backup and destination hashes matched |
| GUI integration | PARTIALLY DELIVERED. WinPE workflow exercised. Complete keyboard-only qualification remains open |
| EFI repair correction | PARTIALLY DELIVERED. Root fixed. Nested paths still block verified ESP preservation |
| W10Val capacity | BLOCKED |
| Linux regression | DELIVERED |
| Windows native validation | DELIVERED in the disposable clone |
| WinPE validation | PARTIALLY DELIVERED. The existing WinPE environment ran the new executable and exported evidence. New ISO build BLOCKED |
| Data integrity | DELIVERED for the verified fixture payload and observed negative cases |
| Workspace hygiene | DELIVERED |
| ClickUp synchronization | BLOCKED by the connector limit. Local roadmap updated |
| Overall Wave 2 delivery | PARTIALLY DELIVERED |
