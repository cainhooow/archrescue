# ArchRescue P0 Windows engineering report

Execution date 2026-11-09, America/Sao_Paulo. Product version 0.0.1. Final tested application source `64bf3a6c92702368a0cb4fe8dc66b9385536d3a4`.

## Initial environment and recovery

Arch Linux x86_64 with VirtualBox 7.2.22r175245 hosted W10Val, UUID `147958bc-a550-4bf1-af59-50981fec149e`. The guest ran Windows 10 Home 22H2 x64, build `19045.2965`, with 4 vCPUs, 6,246 MiB RAM and about 35.9 GiB initially free on C:. The existing loopback SSH alias authenticated as `w10val\archrescue` and provided an elevated token. Git, Rust, MSVC, SDK, ADK and the WinPE add-on were initially absent. [Baseline inventory](../evidence/windows/W10VAL-BASELINE.md)

VirtualBox verified the powered-off snapshot `ARCHRESCUE-PRE-TOOLCHAIN`, UUID `1cc2c8ea-4e47-49ed-954d-3ed9f6330a2f`. The guest restarted and the key-based SSH connection returned. The provisioning record includes recovery steps and each installer source, publisher, SHA-256, exit and verification command. [Provisioning and snapshot recovery](../evidence/windows/W10VAL-PROVISIONING.md), [compatibility decisions](../evidence/windows/TOOLCHAIN-COMPATIBILITY.md)

## Environment

| Gate | Result | Verified outcome |
|---|---|---|
| SSH and authentication | PASS | Noninteractive existing key, expected account and hostname |
| Administration | PASS | Enabled Administrator role, high-integrity token, successful read-only system-volume query |
| Recovery snapshot | PASS | Powered-off snapshot confirmed by UUID |
| Git | PASS | Git 2.56.0.windows.4 and bundle clone |
| Rust | PASS | Rustup 1.29.3, rustc and Cargo 1.99.1, x64 MSVC target, rustfmt and Clippy |
| MSVC | PASS | Visual Studio 2022 Build Tools 17.14.44, x64 compiler, linker, library manager and Dumpbin exercised |
| Windows SDK | PASS | SDK 10.0.26100.0, headers and libraries verified with a compiled probe |
| ADK | PASS | 10.1.26100.9457 deployment tools and matching DISM exercised |
| WinPE Add-on | PASS | Matching setup package, media scripts and AMD64 WinPE image verified |

## Native build and runtime

| Gate | Result | Verified outcome |
|---|---|---|
| Formatting, check, Clippy and release link | PASS | Clean Windows checkout, lockfile retained, MSVC x64 environment |
| PE and dependencies | PASS | Machine 8664, Windows CUI, static CRT configuration and no imported dynamic MSVC CRT |
| Executable smoke and native CLI | PASS | 17 validated CLI checks, including offline Windows discovery, storage API comparisons, verified copy and negative cases |
| Native GUI | PASS | Actual W10Val session 1. The `9e624d2` pt-BR walkthrough verified progress, cancellation and report persistence. The final `64bf3a6` verified reflow, clean close and restart. |
| Workspace Rust suite | FAIL | 26 passed, 2 fixture failures on native Windows. Portable Linux suite passed all 31 tests. |
| In-flight unavailable destination | BLOCKED | Existing test callback cannot rename the directory containing an open file on Windows. Missing destination and partial conflict passed separately. |
| BCD source safety regression | PASS | Final code enumerated an isolated copy and preserved the fixture hash. Shared-physical-disk scratch placement produced signature-only evidence. |

The native executable is 769,536 bytes, SHA-256 `2c292f8f55c6f4d2ec0358d39eba365a91472082e4c7b72c4d5f3a52f3753ca6`. The actual artifact and command, runtime output, fixture identities and test limitations are in the linked records. [Build results](../evidence/windows/WINDOWS-BUILD-RESULTS.md), [native tests](../evidence/windows/WINDOWS-NATIVE-TESTS.md)

## WinPE and VirtualBox

The existing packaging script produced the local ISO with matching ADK and WinPE add-on 10.1.26100.9457. Host and guest SHA-256 agreed at `83e8dad200d60b31a9186d7a5eeaf803de212a35b89e9e0860163350359626f0`. The WIM was committed and the final mount inventory was empty. Public distribution is outside this delivery and requires a Microsoft rights review. [ISO build and checksums](../evidence/windows/WINPE-ISO-BUILD.md)

| Gate | Result | Verified outcome |
|---|---|---|
| ISO creation and checksum | PASS | Script exit 0, 400,230,400 bytes and matching SHA-256 |
| VirtualBox UEFI and WinPE startup | PASS | Final ISO booted at 800×600 and 1280×720 with the network disconnected |
| GUI startup and fallback | PASS | Matched source and PE hash, keyboard navigation, exit 0, fallback console and reopen |
| Storage and candidate detection | PASS | GPT, unmounted EFI, distinct NTFS extents, free space, RAM disk and offline Windows candidate |
| Backup, restoration and independent hashes | PASS | Three files, 262,173 bytes, independent source, copy and restored-file manifests agreed |
| Report and recovery data after restart | PASS | All 21 final QA files, including JSON, text and the copied and restored data, retained their hashes after the next UEFI boot |
| Source base and payload preservation | PASS | Before/after immutable VDI hash and independent file manifests matched |
| Guest-visible write block | BLOCKED | VirtualBox rejected read-only HDD mode. The immutable source base was protected while a disposable guest differencing disk remained writable. |
| Full WinPE GUI workflow and independent human sign-off | NOT TESTED | WinPE CLI exercised the recovery workflow. The screenshots and interaction logs establish the automated GUI observations. |
| QEMU with OVMF | BLOCKED | Required second hypervisor and firmware absent |
| Secure Boot | NOT TESTED | Separate trust configuration and boot evidence outstanding |

[VirtualBox QA configuration, screenshots and evidence](../evidence/windows/VIRTUALBOX-QA.md)

## Git audit

Initial HEAD `b79257e3132c77ad54b2ccc924fcec4ded1a6ee3` on `main`. The reported style commit `73082975b71c975fd36e23e24bf989e9f3d221de` was verified as its direct parent. The initial tree was clean. The Windows source transfer used verified Git bundles, first for the initial revision and then for the final `codex/w10val-native-validation` source.

Two focused source commits followed.

- `9e624d2` changed `crates/diagnostics/src/lib.rs`. It keeps BCDEdit away from the source hive after a direct control command modified the disposable BCD during diagnosis.
- `64bf3a6` changed `apps/boot-shell/src/gui.rs`. Native window sizing and destruction now complete during reentrant Win32 dispatch, allowing the WinPE fallback console to resume.

The roadmap bytes and Cargo.lock remained unchanged. The new evidence and development guide are documentation only. No push, release, external deployment or ClickUp status change occurred. [Development guide](../W10VAL-DEVELOPMENT-GUIDE.md)

The documentation-only delivery commit follows the final executable source. The exact checkout HEAD, source SHA, local commit list and changed file list are recorded in `.agent-tmp/w10val/git-audit-final.log`.

## Open issues

- The two native Rust fixture failures remain visible. The ASCII case-collision case needs a case-sensitive Windows fixture. The in-flight unavailable-volume case needs a controlled disk-loss fixture that works with Windows file sharing.
- The mandatory QEMU/OVMF repeat and guest-visible write blocking remain blocked. Full WinPE GUI recovery, independent human sign-off and Secure Boot trust validation remain outstanding.
- The Windows 10 guest is on an old servicing baseline. The local toolchain is working and W10Val was returned to running with working SSH.
- WinPE redistribution rights and the project source license remain unresolved. The ISO and all Microsoft media stayed local.

## Execution checklist

| Phase | Status | Completed work or next prerequisite |
|---|---|---|
| 0–2 Baseline, snapshot, compatibility | PASS | Inventoried guest, verified recovery UUID and selected supported tools |
| 3–4 Installation and SSH | PASS | Installed and ran all required toolchains. Verified the guest again after restart. |
| 5–6 Source transfer and native build | PASS | Verified committed bundles, rebuilt the exact final source, checked PE and ran it |
| 7 Native tests | FAIL | 26 Rust tests passed and 2 fixture tests failed. All 17 final CLI checks passed. |
| 8 Native GUI | PASS | Walkthrough on the BCD-fix build and lifecycle repeat on the final build |
| 9 WinPE generation | PASS | Existing script, matching tools, checked checksum and empty WIM mount inventory |
| 10 VirtualBox QA | Partial | Final UEFI boot, storage, copy, restore and reboot persistence passed. Other platform and write-blocking requirements remain open. |
| 11 Reproducibility | PASS | Local artifacts, evidence and development guide retained |
| 12 P0 acceptance | BLOCKED | Mandatory open items above |

## Verdicts

| Category | Verdict |
|---|---|
| Windows toolchain | **READY** |
| Windows executable | **VERIFIED** |
| WinPE ISO | **GENERATED** |
| WinPE boot | **VERIFIED** on VirtualBox x64 UEFI |
| ArchRescue P0 | **BLOCKED** |
