# Windows toolchain compatibility

Selected for the Windows 10 Home 22H2 x64 build guest. The checkout declares Rust 1.85 as its minimum and uses the existing lockfile. The validation uses the observed stable MSVC toolchain.

| Component | Selection and prerequisite | Planning allowance |
|---|---|---|
| Git for Windows | 2.56.0.windows.4 x64, official release. Its signed x64 installer supports Windows 10. Required for history-preserving source transfer and clean-tree verification. | Up to 1 GiB including installer |
| VS 2022 Build Tools | Fixed 17.14.44 release, build 17.14.37827.9. Windows 10 Home x64 is covered by the VS 2022 host requirements. The guest has .NET 4.8. Setup manages required prerequisites. | Allow 12 GiB for this selection and staging. Microsoft documents 2.3–60 GB depending on features. |
| MSVC and SDK | VCTools workload plus explicit x64/x86 v143 and SDK 26100 components. The SDK supplies headers and import libraries for the native build. | Included in the Build Tools allowance |
| Rust | Official x64 Windows MSVC Rustup, stable profile with rustfmt and Clippy. Windows 10 meets the Rust target host requirement. | Allow 4 GiB for Rustup, registry cache and workspace build |
| ADK | Microsoft ADK Deployment Tools 10.1.26100.9457. The ADK download lists supported target OS releases separately from the DISM host matrix. Microsoft documents Windows 10 x64 as a host for servicing Windows 11 WinPE with the Windows 11 version of DISM. Actual setup and WIM servicing determine whether this guest works with the selected release. | Allow 3 GiB for deployment tools and staging |
| Windows PE Add-on | Matching 10.1.26100.9457 installer, AMD64 runtime. Microsoft lists base WinPE binaries as 10.1.26100.1 for this kit family. Use the ADK-provided DISM ahead of inbox Windows 10 DISM. | Allow 8 GiB for add-on, build scratch, WIM and ISO |

The September 2026 setup incorporates the documented 26100-series servicing patch. The release supports the intended Windows 11 deployment targets with x64 WinPE. A Windows 11 build VM is the fallback if setup or servicing reports a host limitation. The Windows 10 build is from 2023 and outside the ordinary Windows 10 servicing window. Keep the development and isolated QA roles separate.

Git and Rust are distributed under their respective open-source terms. Microsoft Build Tools, SDK, ADK and WinPE are governed by their accompanying Microsoft terms. WinPE here is for local recovery and deployment validation. Public redistribution of the ISO requires a separate rights decision.

- [Git for Windows release and installer digest](https://github.com/git-for-windows/git/releases/tag/v2.56.0.windows.4), [host requirements](https://gitforwindows.org/requirements.html)
- [Visual Studio 2022 requirements](https://learn.microsoft.com/en-us/visualstudio/releases/2022/system-requirements), [fixed bootstrapper](https://learn.microsoft.com/en-us/visualstudio/releases/2022/release-history#fixed-version-bootstrappers), [workload components](https://learn.microsoft.com/en-us/visualstudio/install/workload-component-id-vs-build-tools?view=vs-2022)
- [Official Rustup](https://rust-lang.org/tools/install/), [Rust MSVC platform requirements](https://doc.rust-lang.org/stable/rustc/platform-support/windows-msvc.html)
- [ADK and matching WinPE downloads](https://learn.microsoft.com/en-us/windows-hardware/get-started/adk-install), [ADK patch status](https://learn.microsoft.com/en-us/windows-hardware/get-started/adk-servicing), [DISM host and target matrix](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/dism-supported-platforms?view=windows-11), [WinPE base versions](https://learn.microsoft.com/en-us/intune/configmgr/core/plan-design/configs/support-for-windows-adk), [WinPE permitted use overview](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/winpe-intro?view=windows-11)
