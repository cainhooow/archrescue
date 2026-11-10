# WinPE ISO build

The corrective run produced a new ISO from source `513c8de`. Both hypervisors and the separate Secure Boot run used its verified hash. The ISO below is historical. [Corrective homologation](../../qa/P0-CORRECTIVE-HOMOLOGATION.md#j-final-artifacts)

**ISO GENERATION PASS.** The final ISO was produced on W10Val by the repository's `packaging/winpe/build.ps1` in a clean Windows checkout. Boot evidence is tracked in [VirtualBox QA](VIRTUALBOX-QA.md).

| Item | Final value |
|---|---|
| Source | `64bf3a6c92702368a0cb4fe8dc66b9385536d3a4` on `codex/w10val-native-validation` |
| Build command | `powershell.exe -NoLogo -NoProfile -NonInteractive -ExecutionPolicy RemoteSigned -File packaging\winpe\build.ps1` from the elevated x64 MSVC and ADK environment |
| Build mode | AMD64 WinPE, ISO output, standard boot media with `BootEx=false` |
| Rust and Cargo | 1.99.1, `x86_64-pc-windows-msvc` |
| MSVC and SDK | MSVC tools 14.44.35207, SDK 10.0.26100.0 |
| ADK and add-on | 10.1.26100.9457, matching x64 WinPE image 10.0.26100 build 1, ADK DISM 10.0.26100.9457 |
| Host artifact directory | `artifacts/windows/64bf3a6/winpe-88bee7b410614c5fb8757d1d78db5088/` |
| ISO | `archrescue-0.0.1-x64.iso`, 400,230,400 bytes |
| ISO SHA-256 | `83e8dad200d60b31a9186d7a5eeaf803de212a35b89e9e0860163350359626f0` |
| Executable SHA-256 | `2c292f8f55c6f4d2ec0358d39eba365a91472082e4c7b72c4d5f3a52f3753ca6` |
| Injected boot.wim SHA-256 | `04de2b205312cfed01a4ab196fac1e382f6bb697f85cee29e511b89bd8f9f100` |
| Script result | Exit 0, guest and host SHA-256 agree, host `file` identified bootable ISO 9660 |
| WIM mount status | DISM listed no mounts before the run and again after completion. The transcript identifies the unique working WIM, mount and commit. |
| Import checks | Dumpbin identified the x64 PE, with static CRT. Every direct non-API-set DLL passed the WIM presence check. The synch API-set loader gate was deferred to runtime. |

The first baseline invocation over SSH failed at `Start-Transcript` integration with a native child console, before mounting any WIM. The same script succeeded from a temporary elevated interactive task with local output. That first generated ISO used baseline `b79257e` and has SHA-256 `7c09562490d99ab5b3569e91455876502d9fd8ad88e4a03e715956b2e444ef99`. The next ISO used `9e624d2` with the BCD isolation fix, SHA-256 `3b53c14a69dae12bfdea23fbf316e8f17ae5909243b925311b32140d9b8c7bb5`. The final ISO adds the WinPE window-close and reflow correction.

The final artifact directory contains `build.log`, `build-info.json`, `pe-headers.txt`, `pe-imports.txt`, `pe-dependents.txt`, `api-set-runtime-verification-required.txt`, and `SHA256SUMS.txt`. The launcher log is `.agent-tmp/w10val/gui-fixed/winpe-build.log`. Baseline transcripts and checksums are under `.agent-tmp/w10val/baseline-winpe/`.

The ISO contains proprietary Microsoft WinPE and is retained for local recovery validation. Public redistribution remains blocked pending a rights review. [WinPE permitted use and limitations](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/winpe-intro?view=windows-11), [project distribution decision](../../decisions/ADR-WINPE-DISTRIBUTION.md)
