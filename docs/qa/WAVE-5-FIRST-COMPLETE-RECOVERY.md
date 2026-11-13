# ARCHRESCUE — WAVE 5 FIRST COMPLETE RECOVERY REPORT

## A. Starting state

- Starting HEAD `bb3218501c8fe1285ed57ea789e9f073e0e6ccf2` on a clean `main`. Work is local on `codex/wave5-end-to-end-recovery`.
- Host Rust 1.99.1, x86_64 Linux, x86_64 Windows GNU and MSVC targets. Native Windows artifacts used host-side GNU linking. VirtualBox 7.2.22 ran the disposable guest.
- WinPE 10.0.26100.1 from the retained `7355215` ISO, SHA-256 `f59deb23036d17263fc58bc4c1fc90cca26f25da0e44f6421355bf0aace0b880`. The shell ran from an auxiliary optical carrier.
- Local installation media `Win10_22H2_BrazilianPortuguese_x64v1.iso`, SHA-256 `0ca714572fcb5b8ce480acc3c93eaf2f34cc6507df5f693cff021bf4a32fac68`. Its 4,853,776,048-byte WIM had SHA-256 `667bf57e11b902b72fc5cb6dccf25f67409542871f9b049d5966e8a8eb299099`. The selected client was Windows 10 Pro x64, index 4. [Media](../../artifacts/evidence/wave5/media.json) · [Host digests](../../artifacts/evidence/wave5/media.sha256)
- Wave 4 recorded 53 WinPE passes and 14 failures. W10Val was below the 15 GiB build floor. Nested ESP normalization for historical repair and the ADK distribution gate were already open.

## B. Deployment executor

The QA-only x64 WinPE build adds a typed layout to the existing clean-install
plan. An Ed25519 registration signed with a per-run host key binds the reviewed
plan, a short-lived operation ID, VirtualBox VM and SATA VDI identity, target GPT
and capacity, empty partition tables, protected GPT disks and a verified backup
manifest. The guest corroborates SMBIOS, storage descriptor and GPT evidence,
requires independent write-protected source and backup disks, and verifies the
WIM hash and index while pinning the image. It repeats the target and authorization
checks immediately before running the generated DiskPart script.

The profile uses an empty 80 GiB VirtualBox GPT disk with 512-byte logical and
physical sectors, a 300 MiB FAT32 ESP, a 16 MiB MSR and NTFS for Windows. WinRE is
outside this profile. The constrained script selects the independently verified
disk and creates the reviewed layout. WinPE reads back partition GUIDs, offsets,
sizes and filesystem mappings. DISM applies the selected WIM with verification.
BCDBoot targets the fresh ESP explicitly and BCDEdit inspects its store. Verified
Windows and EFI markers and the complete step journal are prerequisites for
`awaiting_first_boot`. An exclusive operation directory on the evidence volume
stores step snapshots and tool streams. An interruption leaves the last confirmed
step. [Runbook](WAVE-5-LAB-RUNBOOK.md) · [Plan](../../artifacts/evidence/wave5/plan.json) · [Registration](../../artifacts/evidence/wave5/registration.json)

## C. Real Windows installation

The isolated EFI guest `a39042de-8dde-4018-94df-dae7da4042b7` used Intel AHCI and
a dedicated SATA target. The initial run passed independent preflight and
partition verification, and DISM exited 0. The main thread hit a stack overflow
before confirming the apply step. A separate read-only guest probe opened the
expected registry and PE files. The recorded state remained at `apply_image`,
with preflight and partition as the last confirmed steps. The retry used a fresh
VDI and fresh host authorization after moving the system-directory lookup
buffer to the heap and simplifying marker verification. [Interrupted operation](../../artifacts/evidence/wave5/attempt-1/execution-008.json) · [Stack overflow](../../artifacts/evidence/wave5/attempt-1/execution.err) · [Readability probe](../../artifacts/evidence/wave5/attempt-1/files-readable.png)

The second run completed preflight, partitioning, image application, EFI
configuration and verification. The hypervisor ejected both boot/install ISOs
before first boot. UEFI loaded the new Windows Boot Manager, Windows completed
OOBE, and Explorer started under the disposable local `RecoveryQA` account.
The running guest reported Windows 10 Pro 22H2 x64 build 19045.2965, EFI mode,
and the same GPT ID and serial seen in WinPE. [Deployment record](../../artifacts/evidence/wave5/execution.json) · [VM boot configuration](../../artifacts/evidence/wave5/vm-first-boot.txt) · [Installed OS evidence](../../artifacts/evidence/wave5/installed-boot.json) · [Desktop](../../artifacts/evidence/wave5/installed-desktop.png)

## D. Data recovery

The WinPE source fixture was corroborated by offline Windows markers. ArchRescue
published a manifest-backed backup of five files across Documents, Desktop,
Downloads and Pictures to a separate GPT/NTFS VDI. The fixture included a
Unicode path, nested and empty directories, a 3,100,000-byte binary, a bitmap
and a destination conflict. An independent host NTFS readback matched the
source and backup payload against each manifest size and SHA-256. [Source](../../artifacts/evidence/wave5/source-detected.png) · [Manifest](../../artifacts/evidence/wave5/manifest.json) · [Independent backup readback](../../artifacts/evidence/wave5/independent-backup.json)

The installed Restore Agent consumes the deployment record and checks the
referenced backup manifest during both preview and execution. The restore
fingerprint includes the deployment binding alongside the selected account,
Known Folder identities, mappings and conflict policy. Existing restore
commands remain available without a deployment record.

The agent ran under the destination account's ordinary interactive token and
resolved Documents, Desktop, Downloads and Pictures via the Known Folder API.
Abort preserved the existing file and published no files. Rename restored all
five files, preserving `conflict.txt` and publishing the recovered copy as
`conflict (ArchRescue 1).txt`. A subsequent Skip run skipped the five existing
destinations. Independent NTFS readback matched all three copies for each of the
five files (3,100,118 bytes), confirmed the empty directory and checked the
existing conflict content. PowerShell initially decoded the native UTF-8 output
with the guest OEM code page. The original transcripts and UTF-8 corrected
reports were retained. The actual Unicode filename was verified on all three
filesystems. [Account and Known Folders](../../artifacts/evidence/wave5/installed-profile.json) · [Restore under RecoveryQA](../../artifacts/evidence/wave5/restore-running-account.png) · [Three-way readback](../../artifacts/evidence/wave5/independent-restore.json)

## E. Application integration

The existing Win32 page uses the Wave 4 resource pickers. The QA action
requires a supported clean preview, a selected backup and an evidence folder.
The confirmation identifies the image and GPT disk and states the erase risk.
The worker exposes preflight, partitioning, apply, boot configuration and
verification status. The terminal result includes the evidence path and error
or first-boot handoff state. Installed Windows and the Restore Agent produce
their own evidence after WinPE exits. The successful deployment was driven through the QA CLI. The full GUI journey remains unqualified.

## F. Safety

Host registration independently inspected the VirtualBox machine and VDI
headers, both GPT table copies, attachment slots, protected disk identities and
the configured serial. WinPE corroborated the target and recorded source and
backup read-only status. The lab executor has a separate entry point and
preserves the existing EFI repair code. [Host attestation](../../artifacts/evidence/wave5/host-attestation.json) · [Guest identity](../../artifacts/evidence/wave5/observe-second.json) · [Read-only disks](../../artifacts/evidence/wave5/protected-disks.png)

The protected-window samples matched during the second deployment. Whole-VDI
and NTFS partition digests differed across the broader boot and shutdown
window. The independent file readback found no changes to the protected
recovery data. Full-disk immutability across operating-system startup and
shutdown remains unproven. [Active-window samples](../../artifacts/evidence/wave5/protected-window-second-active.sha256) · [Partition comparison](../../artifacts/evidence/wave5/protected-storage-readback.txt)

## G. Wave 4 failure matrix

All rows refer to the [original WinPE log](../../artifacts/evidence/wave4/windows-winpe-tests.log).
The Wave 5 fixture changes pass explicit persistent source and destination
paths at runtime. Backup previously derived its Windows source from the
compile-time manifest directory. Restore previously used WinPE's temporary
directory for its source. The original exceptions occurred before the asserted
behavior, so the prior log does not establish whether those feature paths work.

| Wave 4 failed test | Observed failure and classification |
|---|---|
| `transaction_refuses_changed_source_destination_conflict_and_corrupt_manifest` | Backup fixture preparation, raw-device open returned Win32 2 at line 562. Source derives from the compile-time path. |
| `transaction_rejects_duplicate_selection_and_moved_operation` | Backup fixture preparation, raw-device open returned Win32 2 at line 614. Same source setup. |
| `transaction_restarts_partial_and_rechecks_published_files` | Backup fixture preparation, path canonicalization returned Win32 2 at line 505. Same source setup. |
| `abort_skip_and_rename_preserve_existing_content` | Restore shared fixture, backup preparation canonicalization returned Win32 2 at line 42. |
| `authorization_is_invalidated_when_known_folder_identity_changes` | Restore shared fixture, same line and error. |
| `cancellation_stops_before_the_next_file` | Restore shared fixture, same line and error. |
| `checksummed_manifest_still_rejects_traversal_and_unknown_versions` | Restore shared fixture, same line and error. |
| `copy_after_preflight_rejects_new_hash_mismatch_and_cleans_staging` | Restore shared fixture, same line and error. |
| `corrupt_and_unsupported_manifest_fail_preflight` | Restore shared fixture, same line and error. |
| `late_destination_conflict_is_never_overwritten` | Restore shared fixture, same line and error. |
| `rejects_windows_directory_junction_escape` | Restore shared fixture, same line and error. |
| `rejects_wrong_sid_traversal_external_mapping_and_unavailable_path` | Restore shared fixture, same line and error. |
| `tampered_and_missing_payload_fail_preflight` | Restore shared fixture, same line and error. |
| `verified_restore_preserves_unicode_empty_and_multichunk_files` | Restore shared fixture, same line and error. |

The installed Windows and WinPE reruns passed all 14 original cases using
separate persistent GPT/NTFS fixtures. The installed suite used an elevated
test token. A first run with the normal desktop
token reached those volumes and failed with access denied on raw-device
queries in fixture preparation. Restore Agent's end-to-end run used the
ordinary token, consistent with its existing account model.

## H. Regression tests

Linux ran 67 default tests through `scripts/test.sh` with a disposable
`/dev/shm` destination, plus 71 tests with all features. Clippy passed for
both x64 Windows targets with all features. The installed Windows GNU test
executables recorded 71 passes and zero failures with the elevated fixture
runner. The WinPE GNU suite recorded 71 passes and zero failures with separate
NTFS fixture paths. The initial ordinary-token test run recorded 44 passes and
27 fixture failures. [WinPE native log](../../artifacts/evidence/wave5/winpe-tests.log) · [Installed Windows](../../artifacts/evidence/wave5/installed-windows-tests.log) · [Initial native run](../../artifacts/evidence/wave5/installed-windows-tests-standard-token.log)

Signature, expiry, token mismatch, plan tampering, changed layout, capacity,
write protection and protected-disk
overlap have focused unit cases. A runtime authorization attempt was rejected
while the guest clock was outside the signed window, before target partitioning.
A changed detached signature was rejected and an unknown GPT ID failed
read-only selection. Host VDI inspection also rejected the populated target.
The three VDI SHA-256 values agreed before and after the final rejection
window. That run's command wrapper emitted an empty exit-code field, so the
recorded failure messages and independent disk comparisons are the available
evidence. The remaining cases below are open. [Rejection logs](../../artifacts/evidence/wave5/negative)

| Requested negative case | Result | Coverage |
|---|---|---|
| Unknown disk | PARTIAL | WinPE read-only target selection refused the unknown GPT ID and VDI hashes matched. The mutation entry point was not reached. |
| Unregistered virtual disk | PARTIAL | WinPE rejected the altered host signature. Runtime disk substitution remains unqualified. |
| Physical host storage | NOT TESTED | Runtime fixture used only disposable base VDIs. |
| Current WinPE boot medium | NOT TESTED | Runtime deployment targeted only the signed GPT VDI. |
| Protected source disk | PARTIAL | Distinct signed roles and guest write-protection gates ran during deployment. A separate refusal fixture remains open. |
| Backup destination disk | PARTIAL | Unit coverage rejects signed role overlap. A separate refusal fixture remains open. |
| Changed disk identity | PARTIAL | Unit coverage rejects a changed GPT ID. The replacement lab VDI required a fresh attestation. |
| Changed partition layout | PARTIAL | Unit coverage rejects a populated observation and host inspection refused the completed target. The guest mutation path was not called with a freshly signed stale layout. |
| Unsupported firmware mode | NOT TESTED | UEFI was the only runtime firmware used for this executor. |
| Unsupported partition scheme | PARTIAL | Unit coverage rejects MBR. |
| Insufficient capacity | PARTIAL | Unit coverage rejects changed capacity and the planner exercises insufficient capacity. |
| Corrupt Windows image | NOT TESTED | Runtime executor was exercised with the verified WIM. |
| Changed WIM hash | PARTIAL | Unit coverage rejects an altered plan hash. A runtime media-change fixture remains open. |
| Invalid image index | PARTIAL | The planner rejects an absent index in its unit fixture. |
| Ambiguous destination selection | PARTIAL | The existing selection unit test rejects duplicate disk IDs. |
| Expired or mismatched authorization | PASS | Guest authorization refusal was observed and target hashes matched across the final rejection window. [Refusal](../../artifacts/evidence/wave5/negative/wrong-token.log) · [Before](../../artifacts/evidence/wave5/negative/before.sha256) · [After](../../artifacts/evidence/wave5/negative/after.sha256) |
| Unsupported encryption state | NOT TESTED | Fresh empty GPT is the qualified runtime profile. |
| Unexpected write-protection state | PARTIAL | Unit coverage rejects changed target write protection. |
| Process execution failure | NOT TESTED | The first-run interruption occurred after a successful child exit. |
| Failed post-operation verification | PARTIAL | An interrupted verification left the journal incomplete. A controlled marker or layout mismatch remains open. [Journal](../../artifacts/evidence/wave5/attempt-1/execution-008.json) |
[Linux default](../../artifacts/evidence/wave5/linux-tests.log) · [All features](../../artifacts/evidence/wave5/linux-all-features.log) · [MSVC](../../artifacts/evidence/wave5/msvc-clippy.log) · [GNU](../../artifacts/evidence/wave5/gnu-clippy.log) · [Clock-window refusal](../../artifacts/evidence/wave5/first-execution.err)

## I. Environment

W10Val reported 12,725,903,360 bytes free on C:. Its existing toolchain was
queried read-only. The Wave 5 guest used disposable media and a registered
105 GiB scratch budget. The retained WinPE ISO and Windows installation ISO
were read-only dependencies. No new WinPE ISO or release ISO was generated.
Both media hashes were rechecked. The disposable VM and media registrations
were removed and the scratch manifest reached `cleaned`. The compact evidence preserves the original manifest digest and removed-path count. Cleanup accounted
for 36,771,082,240 allocated bytes and host available space increased by
44,610,486,272 bytes between measurements.

W10Val appeared in the initial VirtualBox inventory and was absent by the
pre-cleanup inventory. Wave 5 did not issue an unregister command for it.
Its on-disk VM configuration retained the original attachment and snapshot
UUIDs, and the configuration hash captured after the observed change matched
at cleanup. [Initial mapping](../../artifacts/evidence/wave5/w10val-before.txt) · [Dependency recheck](../../artifacts/evidence/wave5/dependencies-verified.txt) · [Scratch manifest](../../artifacts/evidence/wave5/scratch-manifest.json)
[Before](../../artifacts/evidence/wave5/storage-before.txt) · [QA executable digests](../../artifacts/evidence/wave5/qa-executables.sha256)

## J. Git and ClickUp

Implementation commit `ab80e290cd489381ddf8f885b489376af64ed5ef` and host
verification correction `3b692d2`. The local documentation commit contains
the report and compact evidence. ClickUp returned a daily connector limit,
and the browser opened a sign-in page. The roadmap records the updates
pending for the existing tasks. No task status was closed. The 50%
human-review milestone and WinPE/ADK distribution review remain separate gates.

## K. Final acceptance matrix

| Requirement | Result | Evidence |
|---|---|---|
| Deployment executor implemented | PASS | All five stages completed on the second disposable target. [Execution](../../artifacts/evidence/wave5/execution.json) |
| Disposable target attestation | PASS | Host VDI, both GPT tables and SATA mapping agreed with the guest identity. [Attestation](../../artifacts/evidence/wave5/host-attestation.json) |
| Pre-execution identity verification | PASS | The record contains matching SMBIOS UUID, VDI serial, GPT identity, capacity and empty layout. [Execution](../../artifacts/evidence/wave5/execution.json) |
| Image hash revalidation | PASS | The executor completed hash-bound preflight against the optical WIM. [Image and steps](../../artifacts/evidence/wave5/execution.json) |
| UEFI/GPT partition preparation | PASS | DiskPart generated the reviewed FAT32 ESP, MSR and NTFS Windows partition. [Script](../../artifacts/evidence/wave5/executor/partition.txt) · [DiskPart output](../../artifacts/evidence/wave5/executor/tool-00.stdout) |
| Windows image application | PASS | DISM exited 0 and the executor verified Windows files. [Execution](../../artifacts/evidence/wave5/execution.json) · [DISM](../../artifacts/evidence/wave5/executor/tool-01.stdout) |
| Boot configuration | PASS | BCDBoot created the EFI files and BCDEdit read the BCD. [BCD](../../artifacts/evidence/wave5/executor/tool-03.stdout) |
| Partition verification | PASS | Exact partition offsets, sizes, types and GUIDs recorded after preparation and final verification. [Execution](../../artifacts/evidence/wave5/execution.json) |
| Real Windows first boot | PASS | Installed Windows 10 Pro 22H2 build 19045.2965 and Explorer ran from the registered UEFI disk. [Boot record](../../artifacts/evidence/wave5/installed-boot.json) |
| Windows account preparation | PASS | OOBE initialized the local RecoveryQA account and four Known Folders. [Profile](../../artifacts/evidence/wave5/installed-profile.json) |
| Verified source backup | PASS | Five files independently matched the original source and manifest. [Readback](../../artifacts/evidence/wave5/independent-backup.json) |
| Restore Agent execution | PASS | The installed account executed Abort, Rename and Skip. [Restore record](../../artifacts/evidence/wave5/restore-result-renomear.json) |
| Restored file hash verification | PASS | Five files and 3,100,118 bytes matched across all three disks. [Independent readback](../../artifacts/evidence/wave5/independent-restore.json) |
| End-to-end recovery report | PASS | The report links the deployment, first boot, restore plan and independent readback by operation and manifest. [Recovery result](../../artifacts/evidence/wave5/independent-restore.json) |
| GUI deployment integration | PARTIAL | Feature-gated confirmation and backend progress implemented. Full GUI walkthrough pending. |
| Source disk protection | PARTIAL | WinPE reported read-only and active-window VDI SHA-256 was stable. Full VDI hashes changed across boot/shutdown. |
| Backup disk protection | PARTIAL | WinPE reported read-only and payload was verified again after deployment. Full VDI hashes changed across boot/shutdown. |
| Destructive action safety | PARTIAL | One registered target exercised successfully. The complete negative runtime matrix remains open. |
| Negative execution tests | PARTIAL | Focused unit coverage, guest authorization and signature refusals, unknown disk selection, host populated-disk rejection and matching pre/post VDI hashes. [Evidence](../../artifacts/evidence/wave5/negative) |
| Wave 4 failure classification | PASS | All 14 cases are listed in section G with the original log evidence. |
| Linux regression | PASS | 67 default and 71 all-feature tests passed. [Default](../../artifacts/evidence/wave5/linux-tests.log) · [All features](../../artifacts/evidence/wave5/linux-all-features.log) |
| Installed Windows regression | PASS | 71 tests passed in the elevated native fixture run. [Native log](../../artifacts/evidence/wave5/installed-windows-tests.log) |
| WinPE regression | PASS | 71 tests passed on separate persistent NTFS fixtures. [Native WinPE log](../../artifacts/evidence/wave5/winpe-tests.log) |
| Hypervisor end-to-end test | PASS | Diagnosed source, verified backup, deployed, booted, restored and independently read the result. [Joined evidence](../../artifacts/evidence/wave5/independent-restore.json) |
| Workspace hygiene | PASS | The disposable VM and media were closed, dependencies rechecked and scratch marked cleaned. [Closure](../../artifacts/evidence/wave5/scratch-manifest.json) |
| ClickUp reconciliation | BLOCKED | Existing tasks reviewed. The connector reached its daily limit and the browser required sign-in. The roadmap records the pending reconciliation. |
| **Overall Wave 5 delivery** | **PARTIAL** | The first complete lab recovery passed. The full negative runtime matrix, GUI walkthrough and ClickUp reconciliation remain open. |
