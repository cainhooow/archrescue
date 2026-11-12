# ARCHRESCUE — WIN32 TINY-SKIA GRAPHICS MIGRATION REPORT

Local laboratory qualification, November 12, 2026 (America/Sao_Paulo). The owner selected Rust, Win32, tiny-skia, and GDI for the recovery shell. The renderer integrates with the existing application state, worker queue, commands, and authorization paths. The executable is a local x64 MSVC build with the static CRT. [Dependency and license inventory](../decisions/THIRD-PARTY-LICENSES.md)

## Rendering and window lifecycle

- The parent scene and the owner-drawn control rows each have a reusable framebuffer. tiny-skia draws opaque backgrounds, antialiased paths, rounded borders, gradients, focus outlines, navigation symbols, partition bars, and progress. The controls share a small buffer sized for the largest drawn item. An invalidated application scene is regenerated on demand. Expose paints reuse the completed scene and copy the damaged rectangle.
- The top-down 32-bit DIB uses `BI_RGB`. tiny-skia composites transparent coverage against an opaque theme color before the CPU converts RGBA to BGRX. GDI draws queued UTF-16 text over the geometry, then uses `BitBlt` inside `BeginPaint` and `EndPaint`. A device transition can use the clipped `StretchDIBits` path. The GUI owns the memory DCs and bitmaps, flushes GDI before direct DIB writes, and deselects the original GDI objects before release. [Microsoft CreateDIBSection reference](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createdibsection)
- GDI measures and draws bounded text with alignment, wrapping and ellipsis. The theme checks for regular Segoe UI and falls back to Tahoma. The lab WinPE WIM contains regular and bold Tahoma and lacks `segoeui.ttf`. System fonts and the native text controls avoid a separate font runtime. The frame uses physical monitor coordinates. Text scaling responds to monitor DPI up to 125% within the compact layout.
- The first window uses `WS_POPUP` with the active monitor rectangle. `WM_DISPLAYCHANGE` and `WM_DPICHANGED` re-evaluate that geometry. F11 switches the frame style with `SWP_FRAMECHANGED`, retains windowed placement, and restores fullscreen on the active monitor. The application starts from the existing `startnet.cmd`. The console action opens a separate console. The startup console remains available after the GUI exits. [Microsoft monitor positioning guidance](https://learn.microsoft.com/en-us/windows/win32/gdi/positioning-objects-on-multiple-display-monitors)
- The native button and list HWNDs retain hit testing, pressed input, focus, selection, keyboard dispatch, and accessibility roles. Backup and resume use the existing modal confirmation and require a positive choice. The code still validates the reviewed paths and disk identity, performs filesystem work in workers, and only reports completion after the existing readback and hash checks.

[Framebuffer](../../apps/boot-shell/src/gui/renderer/framebuffer.rs) · [Geometry and text](../../apps/boot-shell/src/gui/renderer/mod.rs) · [Window mode](../../apps/boot-shell/src/gui/window.rs) · [Confirmation](../../apps/boot-shell/src/gui/confirmation.rs)

## Design and page migration

Both HTML indexes reference all 84 supplied PNGs. The gallery stylesheet, viewer, and 84 states were reviewed. All 88 design files matched the existing checksum inventory. The report uses the PNG compositions as visual evidence and the live application as the source for Windows findings, capacity, partition extents, status, and report totals. [Owner design gallery](../../design/index.html) · [Technical design reference](../../design/relatorio.html) · [Verified design checksums](../../artifacts/evidence/renderer/design-check.log)

| Surface | Implementation and visual differences |
|---|---|
| Detect | Candidate card on the left, counters and volume summary on the right, disk cards beneath. The actual volume list is abbreviated in the summary and accessible in full through **Detalhes dos discos**. Session integrity and backup summaries appear when the viewport has room. |
| Integrity and disks | Live findings and physical/volume inventory use selectable, themed rows and a scrollable evidence panel. The split becomes stacked at compact widths. The owner’s diagnosis composition informs integrity because the reference predates that page. |
| Backup and checkpoints | The source and destination forms, file columns, selection, preview, progress, checkpoint rows, and verification states use the renderer. Multi-selection and path editing retain native behavior. The preview keeps the fixed action row below the scrollable selection. |
| Report and technician mode | The session metrics, evidence panel, export form, error and outcome cards share the renderer. Technician mode exposes the original detailed records. |
| EFI repair, deployment, and Restore Agent | The common rail, forms, action buttons, cards, and status surfaces use the renderer. The existing planner and handoff handlers retain their behavior. |
| Modal and native scroll controls | Confirmations still show the native dialog chrome. Scrollbar thumbs and combo arrows retain the WinPE system treatment. Typography uses system metrics and icons use small custom line paths. Several reference variants use more individual panels or richer tables. |

## WinPE qualification

The test machine was a network-disconnected VirtualBox UEFI guest with 2 vCPUs, 4 GiB RAM, VBoxSVGA and 3D acceleration disabled. It booted x64 WinPE 10.0.26100.1 from the existing local image with the tested shell inserted. A separate qualification image started a small observer alongside the normal `startnet.cmd` launch. The delivered local ISO uses the normal startup script.

W10Val's system volume reported 12,466,860,032 bytes free (11.61 GiB). The release was linked on Linux with the x64 MSVC target, Rust's LLD, and a scratch copy of the installed Windows SDK 10.0.26100.0 and MSVC 14.44.35207 libraries. The test image preserved the original boot files and rebuilt the UDF filesystem around the updated WIM. The first ISO-9660-only attempt could not resolve the UEFI BCD. The UDF image booted and ran the shell. [Boot-media diagnostic](../../artifacts/evidence/renderer/iso9660-boot-manager-failure.png) · [PE imports](../../artifacts/windows/6445768/pe-imports.txt)

The source was a disposable 50 GiB clone of the W10Val disk chain. Fixture preparation renamed `Windows\System32\services.exe` on that clone and created a 36-byte text file and a 256 MiB zero-filled file. The clone was then attached as immutable, with a disposable VirtualBox differencing disk. The 2 GiB GPT/NTFS destination was a different virtual disk. Its label was `ARRENDER`. The backup and report used that destination. [Fixture preparation](../../artifacts/evidence/renderer/fixture-status.png) · [Disk mapping](../../artifacts/evidence/renderer/fixture-mapping.png)

The initial checkpoint test exposed a WinPE access violation in `DrawTextW`. The new text layer passed a zero-length UTF-16 slice whose pointer had no readable code unit. Commit `00a9d23` guards empty labels before calling GDI and adds a regression test. Checkpoint selection and resume succeeded after the fix. Console testing also found that a new console could be obscured by the fullscreen window. The release opens an interactive command prompt with independent handles and minimizes the GUI. The prompt ran a command in the clean release boot, then Alt+Tab restored the fullscreen shell. [Native crash trace](../../artifacts/evidence/renderer/resume-debug.log)

[Clean ISO startup](../../artifacts/evidence/renderer/clean-iso-startup.png) · [Startup log](../../artifacts/evidence/renderer/startnet-release.log)

### Screens and interaction

| Observation | Evidence |
|---|---|
| Automatic borderless startup at 800×600 and 1280×720 | [800×600 comparison](../../artifacts/evidence/renderer/dashboard-comparison-800.png) · [1280×720 comparison](../../artifacts/evidence/renderer/dashboard-comparison-1280.png) |
| Automatic borderless startup at 1024×768 and 1920×1080 | [1024×768](../../artifacts/evidence/renderer/dashboard-1024x768.png) · [1920×1080](../../artifacts/evidence/renderer/dashboard-1920x1080.png) |
| Native HWND equals the monitor rectangle with the caption absent. F11 restores a caption and the shell lays out a 900×640 window at 1280×720. | [Resize](../../artifacts/evidence/renderer/windowed-resize-1280.png) |
| Mouse hover and selection. Tab, Enter and Space activate navigation. End and the mouse wheel move through the technician findings. | [Hover](../../artifacts/evidence/renderer/hover-navigation-1280.png) · [Technician findings](../../artifacts/evidence/renderer/technician-1280.png) |
| Offline `services.exe` finding and physical inventory | [Integrity comparison](../../artifacts/evidence/renderer/integrity-comparison-1280.png) · [Disks and volumes](../../artifacts/evidence/renderer/disk-inventory-1280.png) |
| Real same-physical-disk rejection and negative-default confirmation | [Rejected destination](../../artifacts/evidence/renderer/same-disk-error-1280.png) · [Confirmation](../../artifacts/evidence/renderer/confirmation-1280.png) |
| Authorized backup, progress, verification and completed checkpoint | [Progress comparison](../../artifacts/evidence/renderer/backup-comparison-1280.png) · [Verified](../../artifacts/evidence/renderer/backup-verified-1280.png) · [Checkpoint](../../artifacts/evidence/renderer/checkpoint-1280.png) |
| Report export, deployment page and verified Restore Agent reference | [Report comparison](../../artifacts/evidence/renderer/report-comparison-1280.png) · [Deployment](../../artifacts/evidence/renderer/deployment-1280.png) · [Handoff](../../artifacts/evidence/renderer/handoff-1280.png) |
| Separate diagnostic console and startup console fallback | [Console action](../../artifacts/evidence/renderer/console-launch.png) · [Fallback](../../artifacts/evidence/renderer/console-fallback.png) |

The reference window's navigation rail occupies about 16% of the inner width. The runtime rail uses 17% at 1280 and 19% at 800 to retain labels and focus targets. The two dashboard columns have nearly equal widths in both. The runtime places the disk-detail action above the fixed footer. The PNG uses taller disk cards with per-partition legends and volume subrows. The runtime uses the observed partition map and a shorter summary, with detailed identities and volumes in the inventory page. The fixture has a GPT destination and an MBR source, so the PNG's unmounted EFI notice does not appear in this run.

The WinPE display driver exposed only its boot mode during the 1280×720 session. The four resolutions were qualified by actual UEFI boots. Windowed resize and fullscreen restoration were exercised in-session. A live display-mode transition, multi-monitor DPI changes, 125% runtime scaling, and screen-reader operation remain unqualified. The cross-target Windows checks and actual WinPE run cover the tested implementation. A separate installed-Windows GUI run, BIOS boot and Secure Boot remain outside this qualification.

## Performance and resource measurements

Startup measures guest process creation through the first synchronous presentation observed by the probe. It excludes UEFI boot, `wpeinit`, and completion of storage inventory. The idle samples lasted five seconds after inventory. CPU is process kernel plus user time. The framebuffer test inspected the actual GDI bitmap dimensions and stride and the allocated tiny-skia pixel slice.

| WinPE desktop | Startup | Idle CPU over 5 s | Peak working set through idle | Main DIB + pixmap bytes |
|---|---:|---:|---:|---:|
| 800×600 | 97.58 ms | 0 ms recorded | 15,757,312 B | 4,046,848 B |
| 1024×768 | 110.68 ms | 0 ms recorded | 18,075,648 B | 6,291,456 B |
| 1280×720 | 124.95 ms | 0 ms recorded | 19,308,544 B | 7,536,640 B |
| 1920×1080 | 118.75 ms | 31.25 ms | 28,487,680 B | 16,711,680 B |

At 1280×720, 20 warmed mouse navigations took 453 ms wall time and 421.875 ms process CPU. Input-to-synchronous-presentation latency was 23.013 ms median and 33.950 ms maximum. The sample includes command handling and native child repaint. Peak working set for the full session was 22,351,872 bytes. The process reported 25 GDI objects before the batch, after the batch, and at the end of the workflow. [1280×720 telemetry](../../artifacts/evidence/renderer/probe-1280x720.log) · [800×600](../../artifacts/evidence/renderer/probe-800x600.log) · [1024×768](../../artifacts/evidence/renderer/probe-1024x768.log) · [1920×1080](../../artifacts/evidence/renderer/probe-1920x1080.log)

The release executable is 2,583,552 bytes. The prior Wave 3 executable was 2,262,016 bytes. The renderer adds seven locked crates including tiny-skia and its path library. The observed PE imports use Win32 system DLLs. The WinPE unit tests passed RGB channel order, alpha compositing, empty-label safety and framebuffer reuse. [Renderer tests](../../artifacts/evidence/renderer/boot-shell-tests.exe.log) · [Dependency tree](../../artifacts/windows/6445768/cargo-tree.txt)

## Regression and artifact identity

- The Linux suite passed formatting, Clippy and 66 tests with the destination on a disposable `/dev/shm` filesystem. MSVC-target Clippy and GNU-target checking passed. In WinPE, three renderer tests and six integrity tests passed. [Linux](../../artifacts/evidence/renderer/linux-tests.log) · [MSVC](../../artifacts/evidence/renderer/windows-msvc-clippy.log) · [GNU](../../artifacts/evidence/renderer/windows-gnu-check.log) · [WinPE integrity](../../artifacts/evidence/renderer/integrity-tests.exe.log)
- The lab recorded a confirmed missing `services.exe`, rejected a destination on the source disk, and defaulted both backup dialogs to **No**. The affirmative action copied and verified 268,435,492 bytes. Resuming the checkpoint reused two verified files. The exported session JSON and text were read from the independent disk, and host SHA-256 readback matched both expected fixture files to the manifest. The handoff records the same manifest fingerprint and requires explicit authorization in the installed Windows agent. [Independent readback](../../artifacts/evidence/renderer/independent-readback.json) · [Session JSON](../../artifacts/evidence/renderer/session-report.json) · [Session text](../../artifacts/evidence/renderer/session-report.txt) · [Handoff JSON](../../artifacts/evidence/renderer/restore-handoff.json)
- Boot repair and deployment screens remained accessible. This run covered the deployment entry and retained planning restrictions. A full deployment, installed-Windows restore, and every reference state remain outside the tested scope.
- The packaged source is commit `64457684be5b4f392b6ed2cadd5fdc6a2c2f4871`. The release executable SHA-256 is `a173db5cbd0ddba58a514dd2aa436a5177363a8576bd97387fb4d3ffc971db6e`. The executable read back from the clean ISO's WIM matched the promoted binary. The WinPE startup log reports the same source commit. [Executable](../../artifacts/windows/6445768/archrescue-boot-shell.exe) · [Local WinPE ISO](../../artifacts/windows/6445768/archrescue-0.0.1-x64.iso) · [SHA-256 sums](../../artifacts/windows/6445768/SHA256SUMS.txt)

## ClickUp reconciliation

The existing tracker entries were reviewed against this evidence. The current wave is recorded locally in this report. The tracker still shows **In progress** for the boot interface, inventory, backup, and QA. Boot repair is also **In progress**. Deployment, Restore Agent, and the first human review remain **To do**. [AR-02 Boot interface](https://app.clickup.com/t/1dfqjx6uw4b) · [AR-03 Inventory](https://app.clickup.com/t/1dfqjx6uw4c) · [AR-04 Boot repair](https://app.clickup.com/t/1dfqjx6uw4d) · [AR-05 Backup](https://app.clickup.com/t/1dfqjx6uw4e) · [AR-06 Deployment](https://app.clickup.com/t/1dfqjx6uw4f) · [AR-07 Restore Agent](https://app.clickup.com/t/1dfqjx6uw4g) · [AR-14 QA](https://app.clickup.com/t/1dfqjx6uw4p) · [AR-QA-M50 Human review](https://app.clickup.com/t/1dfqjx6uyrk)

## Workspace cleanup

The registered run reserved a 60 GiB budget. Its manifest recorded 55,504,265,216 allocated bytes before closure. The immutable Windows fixture and historical WinPE ISO matched their pre-run SHA-256 values. The guest shut down, its disposable VM and media registrations were removed, and W10Val's attachments and recovery snapshots were verified. The host removed the owned overlays, disk copies, SDK copies, WIM and ISO staging, and superseded builds. Filesystem free space increased by 46,599,233,536 bytes, about 43.4 GiB. The clean ISO, executable, dependency notices, and test evidence are retained locally. [VM cleanup](../../artifacts/evidence/renderer/vm-cleanup.log) · [Source protection](../../artifacts/evidence/renderer/source-protection.log) · [Storage measurement](../../artifacts/evidence/renderer/storage-cleanup.log) · [Closed run manifest](../../artifacts/evidence/renderer/scratch-manifest.json)
