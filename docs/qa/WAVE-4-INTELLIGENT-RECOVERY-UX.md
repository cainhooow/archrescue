# ARCHRESCUE — WAVE 4 INTELLIGENT RECOVERY UX REPORT

12 November 2026 · Local branch `codex/wave4-guided-recovery` · Overall delivery **PARTIAL**

## A. Architecture

- The shell builds a generation-numbered resource catalog alongside the existing diagnostic report. It reuses storage inventory, mounted partition identities, Windows detection, diagnostic ESP associations and backup checkpoint validation. Startup, explicit refresh and supported device broadcasts use the same worker. An open modal keeps its state while refresh waits.
- The picker displays the catalog and opens folders on a worker. It returns a typed selection bound to a disk ID, a storage location or a backup manifest fingerprint. Optical media use the mount identity and serial for selection. Deployment hashes the image before planning and checks that hash again. Technical paths stay in the domain objects. Friendly paths are used for display only.
- The recovery session retains Windows, backup and per-page destinations. A still-matching Windows candidate can survive a drive-letter change on refresh. Other stale selections are rejected on revalidation and may require choosing again. Backup authorization binds the reviewed source, destination and selected folders. EFI and deployment still use their existing planning and execution boundaries.
- The console runs fixed system executables on a worker with redirected stdout and stderr, no shell interpretation, and a bounded output buffer. The session records the command, start time, exit status, cancellation and launch or capture errors.

## B. Before and after

The [baseline](WAVE-4-UX-BASELINE.md) was compiled from the actual local source and official design. It counted path fields and identified the required actions. No complete pre-change runtime walkthrough was recorded, so there is no measured total-click comparison. Runtime counts below count logical selections or activations, excluding focus movement. A path field may be optional in the inherited workflow.

| Journey | Before | Observed Wave 4 behavior | Measured Wave 4 |
|---|---|---|---|
| Save documents | Two path fields and manual source browsing | A Windows candidate and Documents were discovered, then the operator selected the folder, chose a destination and confirmed the plan | Ten logical actions including opening the page, zero typed paths, one safety confirmation |
| Choose backup destination | One path field | Open picker, select volume, open volume, select ArchRescue folder, confirm | Five logical actions in the pointer walkthrough, zero typed paths |
| Locate EFI and prepare repair | ESP path when unresolved, plus preservation path | The catalog supplies candidates and the same picker handles preservation | No ESP fixture in this run, count unavailable |
| Locate installation media | One path field | Open deployment, open media picker, choose the discovered WIM, confirm inspection | Four logical actions, zero typed paths or indexes for initial edition |
| Choose prior backup | One path field | Open Verify, open backup picker, select discovered operation, confirm and verify | Five logical actions including fresh payload verification, zero typed paths |
| Prepare restore | Two path fields | Choose discovered backup and export location, then reread and publish the handoff | Eight logical actions including opening the page, zero typed paths. Account and mapping remain in the installed agent |
| Export report | One path field | Open Reports, open picker, select independent volume, confirm and export | Five logical actions, zero typed paths |
| Run diagnostics | One external console window | Open Console and execute the default diagnostic | Two logical actions, zero external windows during execution |

The source fixture had one corroborated candidate. The guided backup therefore required no Windows re-selection. The restore handoff and report walks used separate application sessions. The shell can reuse a previously selected backup while navigating within one session.

## C. Resource picker

The Win32 modal uses the shell framebuffer, owner-drawn list rows, selection colors, keyboard focus and custom scrollbars. It supports Windows, ESP, GPT deployment disk, source directory, WIM/ESD file, backup operation, and destination directories for backup, preservation, reports, plans, logs and restore references. Back follows the navigation stack. The current path, row details and search remain visible above the action row.

WinPE exercised pointer selection and keyboard navigation with Tab, Shift+Tab, arrows, Enter to open folders, Space to confirm, and Escape to dismiss. The read-only fixture disk showed its specific restriction. Destination validation also checks source separation, persistent storage identity, basic filesystem access and free space. Explicit folder creation uses a validated single-component name and an exclusive filesystem creation call. Folder creation, multi-step breadcrumbs, no-letter volumes, device removal, traversal and insufficient-capacity behavior need dedicated end-to-end picker coverage. [Destination browser](../../artifacts/evidence/wave4/destination-browser.png) · [Read-only disk](../../artifacts/evidence/wave4/readonly-destination.png)

## D. Backup and discovery

Two disposable 384 MiB GPT/NTFS disks supplied the source and destination. The source contained offline detection markers copied from WinPE, a fixture profile and a 29-byte Documents file. The UI displayed the corroborated candidate and suggested Documents. Conventional local folders require a readable profile marker and a readable folder. Redirected Known Folders and arbitrary offline profile metadata are outside the current discovery scope. The existing transaction engine computed the reviewed count and bytes, used its checkpoint, and published the payload after SHA-256 verification.

After the shell restarted, the picker found the backup in the conventional ArchRescue directory and identified the completed status as recorded pending a fresh read. The Verify page reread the payload. Host extraction matched source bytes, published bytes and the manifest SHA-256. Linux also exercised actual process interruption, checkpoint discovery, resume, restore and independent hash comparison. The index bounds the known-directory scan and manifest reads. A custom operation directory remains selectable by browsing. [Suggested folder](../../artifacts/evidence/wave4/windows-and-documents.png) · [Review](../../artifacts/evidence/wave4/backup-preview.png) · [Confirmation](../../artifacts/evidence/wave4/backup-confirmation.png) · [Completed](../../artifacts/evidence/wave4/backup-completed.png) · [Rediscovered backup](../../artifacts/evidence/wave4/backup-rediscovered.png) · [Independent readback](../../artifacts/evidence/wave4/independent-readback.json)

## E. EFI and deployment

EFI uses the diagnostic association for the selected installation. A sole accessible ESP on the same GPT disk is a suggestion with an unverified BCD reference. Multiple candidates remain ambiguous and the planner retains its identity and preservation checks. The existing nested ESP canonicalization blocker and the laboratory-only BCDBoot authorization remain in force. This wave did not qualify a matching or ambiguous ESP fixture.

WinPE discovered the mounted Windows 10 installation image and the operator selected it in the picker. DISM read editions and architecture and the existing inspector hashed the 4,853,776,048-byte WIM. A separate file browse supports WIM/ESD. ISO contents are available through mounted media. The deployment page can select a GPT disk and request the existing plan, and installation execution remains unavailable. The Wave 4 walkthrough stopped at the inspected image and edition list. [WIM and editions](../../artifacts/evidence/wave4/media-inspected.png)

## F. Restore and reports

The WinPE restore page selected the rediscovered backup and a persistent export destination, independently read the payload and exported the reference. Host extraction matched its fingerprint to the backup manifest. Restoration in the installed Windows account still uses Restore Agent and its Known Folder and conflict policies. A guided account picker, folder mapping, conflict review and execution through a handoff consumer remain open.

The guided report destination produced both JSON and readable text. The backend reread both exports, and the host independently extracted and hashed them. [Restore reference](../../artifacts/evidence/wave4/restore-reference.png) · [Handoff JSON](../../artifacts/evidence/wave4/restore-handoff.json) · [Report JSON](../../artifacts/evidence/wave4/report.json) · [Report text](../../artifacts/evidence/wave4/report.txt)

## G. Integrated console

The current actions are network configuration, volume mount points and process listing. The runner captures stdout and stderr separately, collects the exit code, limits each stream to 128 KiB, decodes UTF-8, UTF-16LE with BOM or the OEM code page, and polls the child for cancellation. The last 20 visible results can be recalled. Copy, clear and independently reread log export are available. This is a fixed-action runner. Interactive terminal emulation and arbitrary command input are outside its implementation. The shell bootstrap returned to the emergency command prompt when the GUI closed, and Technician Mode retains the external console action.

WinPE displayed the real output of `ipconfig /all` and exit code 0. The final source revision repeated that run. Stderr, Unicode from actual child output, output limits, cancellation, process launch failure and log export need runtime coverage. [Integrated console on final revision](../../artifacts/evidence/wave4/console-final-revision.png) · [Emergency prompt](../../artifacts/evidence/wave4/emergency-console.png)

## H. Visual acceptance

Both HTML references rendered and all 84 PNGs were inspected. The picker borrows the existing dimmed confirmation surface and the explorer's row, focus and scrollbar patterns. The official package has no integrated console mockup. The console uses the same navigation, palette, typography, controls and footer. The WinPE screenshots show the actual 1024×768 workflows. The 800×600 boot attempt did not reach a usable shell in the sampling window. The 1280×720 and 1920×1080 checks did not complete. Full responsive workflow and DPI qualification remain open. [Gallery](../../artifacts/evidence/wave4/reference-gallery.png) · [Design report](../../artifacts/evidence/wave4/reference-report.png)

## I. Safety and tests

- **Linux:** 67 tests, Rustfmt and workspace Clippy passed. The test destination used a disposable directory on the separate `/dev/shm` filesystem. Release CLI acceptance and interrupted transaction recovery passed. [Linux suite](../../artifacts/evidence/wave4/linux-tests.log) · [CLI](../../artifacts/evidence/wave4/linux-cli.log) · [Resume](../../artifacts/evidence/wave4/resume-trace.json)
- **Cross-target:** Workspace Clippy passed for x64 MSVC and GNU. The host linked both MSVC release executables using the installed SDK and VC import libraries. [MSVC](../../artifacts/evidence/wave4/msvc-clippy.log) · [GNU](../../artifacts/evidence/wave4/gnu-clippy.log) · [Link log](../../artifacts/evidence/wave4/final-build.log)
- **Windows executables in WinPE:** 53 tests passed and 14 failed in backup and restore fixture setup or transaction preparation. The source paths resolved through the WinPE RAM environment or the Linux compile-time workspace path, where the tests could not establish persistent source identity. A restore retry with redirected environment variables still failed during canonicalization. The test failure record is preserved alongside the successful two-disk GUI workflow. Installed Windows regression remains untested for this revision. The real GUI operation used two independent GPT volumes and reached verified publication. [Native WinPE suite](../../artifacts/evidence/wave4/windows-winpe-tests.log)
- **Source protection:** DiskPart reported the source disk read-only before the exercised workflow. An early comparison crossed guest startup and showed a whole-VDI difference. During the protected approval and copy window the whole-VDI SHA-256 matched before and after. Independent host extraction also confirmed equality of the source fixture file and backup payload. [DiskPart evidence](../../artifacts/evidence/wave4/source-readonly.png) · [Immediately before copy](../../artifacts/evidence/wave4/source-active-after.sha256) · [After copy](../../artifacts/evidence/wave4/source-verified-after.sha256)

## J. Environment and Git

The starting revision was `7355215a52e648987f74e5cdccd7b106dab3ce10`. The baseline is `171aa1e`. The implementation and final Windows release binaries are from `02c574935b1cabeb4534b900064a248681c7b5b3`. Most workflow screenshots were collected on the immediately preceding validation build. The final revision removed hidden controls and adjusted labels, spacing and notification behavior. WinPE confirmed the final executable build ID and reran the integrated diagnostic.

| Windows artifact | SHA-256 |
|---|---|
| Boot shell | `1edab1000ae1bfbb2a009bb74a6d2f1ea68f5617663d338e91dad48c178d8dee` |
| Restore Agent | `b78d303cb4d754053bd2a39835be18a9e0e370b108ff3a537139d116aa736a0a` |
| Reused WinPE boot ISO | `f59deb23036d17263fc58bc4c1fc90cca26f25da0e44f6421355bf0aace0b880` |
| Disposable auxiliary carrier with final executable | `849f9060eb3d7dde40ba1421de708b08490c4cf0f04c0ef311ccae19d084e876` |

W10Val measured 12,730,220,544 bytes free after SDK readback, below the 15 GiB floor. The guest was shut down after reading the toolchain and its attachment and snapshot UUIDs matched the initial mapping. No new bootable WinPE ISO was built. The owner design checksum matched all 88 tracked files. Seven existing ClickUp descriptions were updated and verified by readback. The first formal review remains scheduled at the existing evidence-backed 50% milestone. [Windows build ID](../../artifacts/evidence/wave4/winpe-final-id.png) · [W10Val mapping](../../artifacts/evidence/wave4/w10val-final.txt) · [ClickUp reconciliation](../../artifacts/evidence/wave4/clickup-reconciliation.json)

The registered run reserved 8 GiB and accounted for 2,290,737,152 bytes before cleanup. Host available space rose from 297,816,068,096 to 298,946,244,608 bytes, a measured gain of 1,130,176,512 bytes. The disposable VM was powered off and unregistered. All scratch disk and optical registrations were closed, the retained evidence was verified, and the run manifest reached `cleaned`. [VM closure](../../artifacts/evidence/wave4/qa-vm-before-cleanup.txt) · [Storage measurements](../../artifacts/evidence/wave4/storage-cleanup.log) · [Scratch manifest](../../artifacts/evidence/wave4/scratch-manifest.json)

## K. Remaining work

- Qualify the disk and volume picker under device removal and remount, long and Unicode paths, no-letter volumes, blocked permissions and insufficient space. Add explicit stale state for every saved destination, full navigable breadcrumbs and file filters for callers beyond Windows media.
- Add richer offline profile metadata and Known Folder redirection. Exercise multiple Windows installations and matched, ambiguous, unmounted and changed ESPs. Keep preservation and repair execution tied to the existing safety gates.
- Complete the guided installed-Windows restore account, folder and conflict flow. The WinPE handoff has runtime evidence. Deployment plan selection, full execution and post-installation restoration require separate evidence.
- Qualify console cancellation, output cases and export, screen-reader behavior, high DPI and all responsive states. Repeat the native test suite on installed Windows with dedicated GPT fixture storage.

## Acceptance matrix

| Requirement | Result | Observed result or open coverage |
|---|---|---|
| Unified resource discovery | PARTIAL | Real disk, volume, Windows, media and backup discovery. Hot-plug and stale-device fixtures remain open |
| Friendly path presentation | PASS | Supported namespace forms covered by tests. WinPE rendered drive paths and relative names |
| Universal resource picker | PARTIAL | Several typed modes reached real backends. Additional negative cases and browse modes remain open |
| Disk/volume selection | PARTIAL | Independent GPT volume selected. Deployment disk workflow remains unqualified |
| Folder selection | PASS | WinPE selected an actual destination subdirectory and used it for backup |
| File/media selection | PASS | Detected WIM selected and inspected in WinPE |
| Custom modal design | PASS | Themed picker and confirmation observed with native keyboard and pointer interactions at 1024×768 |
| Save Files workflow | PASS | Guided source, selection, review, explicit authorization and verified publication |
| Automatic Windows source detection | PASS | Corroborated a candidate from the offline marker fixture and suggested Documents |
| Guided backup destination | PASS | Read-only disk explained. Independent destination selected and used |
| Backup auto-discovery | PASS | Completed operation found after application restart |
| Backup integrity classification | PARTIAL | Validated manifest, completion recorded, pending reread and independently verified exercised. Corrupt/inaccessible/unsupported GUI fixtures open |
| Automatic EFI association | PARTIAL | Guided same-disk GPT suggestion implemented. Matching ESP runtime open |
| EFI ambiguity handling | PARTIAL | Existing ambiguous-plan guard retained. Runtime ambiguity fixture open |
| Guided repair preservation | PARTIAL | Picker and independent-disk validation integrated. Actual EFI preservation remains blocked |
| Windows media discovery | PASS | Detected mounted Windows 10 WIM with real editions |
| Deployment UI simplification | PARTIAL | Media and edition workflow exercised. Full plan journey remains open |
| Guided restore selection | PARTIAL | Backup chosen and verified handoff exported. Account mapping and conflicts remain in the installed agent |
| Guided report export | PASS | JSON and text exported, reread and independently extracted |
| Integrated diagnostic console | PARTIAL | Real command, captured output and exit code. Remaining diagnostic cases need runtime coverage |
| Emergency console fallback | PASS | WinPE bootstrap returned to the command prompt after GUI closure |
| Shared recovery session | PARTIAL | Shared Windows and backup selections. Stale-device coverage and richer notifications remain open |
| Responsive design | PARTIAL | Full workflow evidence at 1024×768. Multi-viewport and DPI walkthrough pending |
| Keyboard accessibility | PARTIAL | Tab, Shift+Tab, arrows, Enter, Space and Escape exercised. Full keyboard and assistive-technology qualification pending |
| Windows native validation | NOT TESTED | Installed-Windows suite pending. Both cross-target Clippy checks passed |
| WinPE runtime validation | PARTIAL | Core GUI workflows passed. WinPE test executables recorded 53 passes and 14 failures |
| Backup regression | PARTIAL | Linux passed and real WinPE copy verified. Three WinPE fixture-based backup tests failed |
| Source data protection | PASS | Read-only source during copy, equal whole-VDI digests across the copy and independent file equality |
| Workspace hygiene | PASS | Disposable VM and owned scratch cleaned. W10Val media and snapshots preserved |
| ClickUp reconciliation | PASS | Seven existing descriptions updated and read back |
| Overall Wave delivery | PARTIAL | Guided recovery workflows run in WinPE. Remaining work and runtime boundaries are listed above |
