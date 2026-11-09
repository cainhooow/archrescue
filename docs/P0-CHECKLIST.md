# P0 implementation checklist

- [x] Inspect initial repository/environment and read current project references.
- [x] Preserve original roadmap and establish local Git history.
- [x] Establish architecture and explicit safety boundary.
- [x] Implement Rust contracts and Windows read-only platform adapters (runtime blocked).
- [x] Implement offline candidate discovery, EFI/header/BCD diagnostic path and explorer.
- [x] Implement selective content copy, integrity, cancellation and local reports.
- [x] Implement native pt-BR shell source and runnable portable CLI.
- [x] Prepare local WinPE build kit, import checks and VM acceptance instructions.
- [x] Execute formatting, native/MSVC-target lint, 31 tests and independent CLI fixture checks.
- [x] Perform a separate adversarial review pass and record remaining risks.
- [x] Record feature matrix, hashes, limitations, dependency licenses and handoff.
- [ ] Build Windows executable and inspect actual imported DLLs (missing MSVC linker/SDK).
- [ ] Execute WinPE build script and generate ISO (Windows/ADK/WinPE missing).
- [ ] Exercise GUI and all ten capabilities against actual Windows VM data.
- [ ] Boot and verify backup persistence in two VM platforms; test Secure Boot.
- [ ] Obtain external independent QA sign-off and resolve distribution/license decisions.
