# W10Val baseline

Audit date 2026-11-09, America/Sao_Paulo. This record describes the VM before the toolchain snapshot.

| Item | Observed baseline |
|---|---|
| Host | Arch Linux x86_64, VirtualBox 7.2.22r175245 |
| Development VM | W10Val, UUID `147958bc-a550-4bf1-af59-50981fec149e`, initially running in the VirtualBox GUI |
| Guest | Windows 10 Home 22H2 x64, build `19045.2965`, Brazilian Portuguese |
| VM resources | 4 vCPUs, 6,246 MiB RAM, BIOS, VBoxSVGA, 256 MiB VRAM |
| Available guest memory | 3,720,972 KiB available out of 6,395,444 KiB visible during the complete audit |
| System storage | 50 GiB VirtualBox VDI, SATA port 0, MBR, disk 0, serial `VB9a90222d-5ea197df`. C: NTFS had 38,530,224,128 bytes free out of 53,684,989,952 |
| Optical media | SATA port 1, VirtualBox Guest Additions 7.2.22 ISO |
| SSH | `ssh -o BatchMode=yes w10val whoami` returned `w10val\archrescue`. `hostname` returned `W10Val`. Existing NAT forward binds only to 127.0.0.1:2222. |
| Elevated token | Administrator role enabled, integrity SID `S-1-16-12288`. Read-only `fsutil dirty query C:` returned exit 0 and reported a clean volume |
| Interactive desktop | Explorer was running in session 1 |
| PowerShell and .NET | Windows PowerShell 5.1.19041.2673 Desktop, .NET Framework 4.8 (registry release 528372) |
| Inbox tools | OpenSSH Server, curl.exe 8.0.1, DISM file version 10.0.19041.844 |
| Build tool discovery | No Git, Rustup, Cargo, Rust compiler, MSVC, SDK or ADK/WinPE registration or files in the standard discovery paths. `vswhere.exe` was absent. |
| Package manager discovery | No `winget`, Chocolatey or Scoop resolved in the SSH environment |
| Installer staging | The user TEMP directory is on C:. The selected minimal toolchains and local packaging work fit the initial 35.9 GiB free-space budget. |
| TLS | Guest curl with certificate validation received HTTP 200 from official Visual Studio, Rust, Git for Windows and ADK sources and the crates.io index. |
| Servicing | No CBS reboot marker, Windows Update reboot marker or pending file renames. Windows Update and BITS were running, with recent update scans/downloads. The last observed CBS transaction finalized with success and no reboot requirement before graceful shutdown. |

Activation was outside the build prerequisites. The SSH client emitted a warning about the negotiated key exchange lacking post-quantum protection. The existing loopback forward and key-based login were used throughout.

The initial checkout was clean on `main` at `b79257e3132c77ad54b2ccc924fcec4ded1a6ee3`. The style commit `73082975b71c975fd36e23e24bf989e9f3d221de` exists and is its direct parent. Both local branches pointed at the same HEAD. A read-only `git ls-remote` found that SHA on remote `main` and no remote `codex/source-style-remediation` branch.

Raw local evidence is in `.agent-tmp/w10val/baseline.log`, `snapshot.log` and `native-disk-attachments.log`.
