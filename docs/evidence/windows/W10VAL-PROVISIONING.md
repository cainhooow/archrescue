# W10Val provisioning

## Recovery checkpoint

| Item | Result |
|---|---|
| VM | W10Val, `147958bc-a550-4bf1-af59-50981fec149e` |
| Snapshot | `ARCHRESCUE-PRE-TOOLCHAIN`, `1cc2c8ea-4e47-49ed-954d-3ed9f6330a2f` |
| Creation | PASS. The guest shut down normally and VirtualBox confirmed a powered-off snapshot on 2026-11-09. |
| Guest return | PASS. Passwordless `whoami` and `hostname` succeeded after start. |
| Recovery | Preserve needed guest artifacts first. With W10Val powered off, verify the snapshot with `VBoxManage snapshot W10Val list --machinereadable`, then restore only that UUID if rollback is required. Reboot and verify SSH. Restoring discards guest changes made after the snapshot. |

The system disk remained the boot disk. Two new disposable SATA VDIs were added after the snapshot, and only those VDIs were initialized and formatted for test data. The [baseline](W10VAL-BASELINE.md) describes the pre-snapshot VM.

## Installers and verification

All five bootstrapper processes returned exit 0. Each installed tool was located and exercised from the guest. The checksums below identify the downloaded bootstrapper files.

| Component | Version and installer origin | SHA-256 | Installation and verification |
|---|---|---|---|
| Git | 2.56.0.windows.4, [official Git for Windows release](https://github.com/git-for-windows/git/releases/tag/v2.56.0.windows.4) | `4961a4a8cc778d8b6e00a5819e5fe4d078b754555aff7aedc2d77a5d74c2de80` | Inno Setup `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /SP-` with a log. Valid Authenticode signer Johannes Schindelin, and digest matched the release. SSH `git --version` and the bundle clone worked. |
| VS 2022 Build Tools | 17.14.44, build 17.14.37827.9, [official fixed bootstrapper](https://learn.microsoft.com/en-us/visualstudio/releases/2022/release-history#fixed-version-bootstrappers) | `5dc747350fb0589f908915c7b5fa52ec22d22b37ae2e5f4964d83519667498b2` | Valid Microsoft Authenticode signature. `--quiet --wait --norestart --nocache` with VCTools, x64/x86 MSVC and SDK 26100. `vswhere` reported complete, launchable, and no reboot required. |
| MSVC and SDK | MSVC directory 14.44.35207, compiler 19.44.35229.0, linker 14.44.35229.0, SDK 10.0.26100.0 | Same Build Tools bootstrapper | x64 `VsDevCmd.bat`. Located and ran `cl`, `lib`, `link` and `dumpbin`. Checked `Windows.h`, `Kernel32.Lib`, `libucrt.lib` and `libcmt.lib`. Compiled and ran a standalone SDK probe and built the Rust PE. |
| Rust | Rustup 1.29.3, rustc and Cargo 1.99.1, [official Windows x64 Rustup distribution](https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe) | `38027d51d69a5d05493627a0634dbea52d6cd793c8f072d2373d3f4163eec7db` | Rustup has no Authenticode signature. The Rust-hosted SHA-256 matched from guest and host. Installed `stable-x86_64-pc-windows-msvc` with the minimal profile, rustfmt and Clippy. Verified `rustc -Vv`, `cargo --version`, `rustup show` and the installed target. |
| ADK | Deployment Tools 10.1.26100.9457, [Microsoft download](https://go.microsoft.com/fwlink/?linkid=2289980) | `ac6a930fdb5c2980ba5fefe606d47edaafcf5f647b4337411500d158ea77300f` | Valid Microsoft Authenticode signature. `/quiet /norestart /features OptionId.DeploymentTools` with setup logging. Verified `DandISetEnv.bat`, DISM 10.0.26100.9457 and Oscdimg 2.56. |
| WinPE Add-on | Installer 10.1.26100.9457, [Microsoft matching download](https://go.microsoft.com/fwlink/?linkid=2289981) | `d4de67acc83de253ccc941dd0590808d83726bdd8ea768a374a7beaff7949d6a` | Valid Microsoft Authenticode signature. `/quiet /norestart` with setup logging. Verified `copype.cmd`, `MakeWinPEMedia.cmd` and amd64 en-US `winpe.wim`. ADK DISM read x64 WinPE 10.0.26100, service pack build 1. |

The account used its existing elevated SSH token. Git and Rust were resolved from their installed directories in the build process. Visual Studio and ADK environment changes were scoped to the build shell. The successful installer exits and `vswhere` status required no installer-triggered guest reboot.

Raw evidence is stored locally under `.agent-tmp/w10val/`. See `snapshot.log`, `native-disk-attachments.log`, `installer-downloads.log`, `git-install.log`, `vs-install.log`, `rust-install.log`, `adk-install.log` and `installer-logs.zip`.
