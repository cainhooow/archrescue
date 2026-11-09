# ADR: local WinPE build kit, publication blocked

Date: 2026-11-09. Status: implemented build-kit approach; redistribution rights unresolved.

ArchRescue-owned Rust source is separated from Microsoft tools and runtime. This repository contains no Microsoft binaries, Windows image, third-party utility bundle, product key, or license bypass. No final project license has been selected; no GPL approval is assumed. Until the owner selects a license, the source should not be described as already released under an open-source license.

Microsoft describes WinPE as a deployment/recovery environment with a separate ADK add-on. Its application guidance discusses OEM deployment/recovery apps, which does not establish a general public ISO redistribution grant. The exact terms accompanying the builder's ADK/WinPE release must be reviewed and retained before public distribution. This session did not obtain or accept those terms for the user. Public ISO redistribution is BLOCKED; local builds require legitimately obtained dependencies and applicable rights.

## Build policy

Use a Windows x64 host, Microsoft Visual Studio C++ build tools and Windows SDK, Rust MSVC target, and official ADK Deployment Tools plus matching WinPE add-on. The local script uses copype, DISM on its own working WIM, and MakeWinPEMedia /ISO. It never uses /UFD or changes target disk partitions. No Windows image is downloaded by the script. Pin and record ADK/SDK/firmware versions in each qualified lab build; the workflow is repeatable, but byte-for-byte ISO reproducibility has not been demonstrated (ADK tools may embed timestamps).

Static CRT is configured for the MSVC target. The build records actual import/dependency output and checks directly imported DLLs against the WIM. API-set resolution and transitive runtime availability still require actual execution. GNU-target checks on Linux are type checks, not the release toolchain. MSVC linking and runtime evidence remain mandatory.

## Secure Boot

The documented MakeWinPEMedia /bootex option selects Windows UEFI CA 2023 boot files and requires a supporting ADK. Firmware trust databases, revocations, matching boot components, and actual UEFI execution determine compatibility. Leave Secure Boot NOT TESTED until verified in a VM configured with the relevant trust keys. Do not infer compatibility from file generation, signatures alone, or successful non-Secure-Boot startup. Do not disable Secure Boot as a claimed fix.

## Sources reviewed

- [Microsoft WinPE overview](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/winpe-intro?view=windows-11)
- [WinPE application guidance](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/winpe-create-apps?view=windows-11)
- [Download and install ADK](https://learn.microsoft.com/en-us/windows-hardware/get-started/adk-install)
- [MakeWinPEMedia options](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/makewinpemedia-command-line-options?view=windows-11)
- [wpeinit / Startnet startup](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/wpeinit-and-startnetcmd-using-winpe-startup-scripts?view=windows-11)

This is an engineering distribution decision, not a determination that third-party redistribution rights have been granted.
