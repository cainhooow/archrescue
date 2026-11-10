# Native Windows build results

| Gate | Result | Evidence |
|---|---|---|
| Source transfer and clean checkout | PASS | Complete Git bundle cloned to Windows `codex/w10val-native-validation` at `64bf3a6c92702368a0cb4fe8dc66b9385536d3a4`. Windows and Linux bundle SHA-256 matched `2bc90fc6393c908066b12005d9e6295b1e27acf37fbecc6719a9282bd421f066`. |
| Formatting | PASS | `cargo fmt --all --check` |
| All-target compilation | PASS | `cargo check --workspace --all-targets --locked --target x86_64-pc-windows-msvc` |
| Lint | PASS | `cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings` |
| Release link | PASS | `cargo build --release --locked --target x86_64-pc-windows-msvc -p archrescue-boot-shell` |
| PE architecture and imports | PASS | Dumpbin reported PE machine `8664` x64 and Windows CUI subsystem. The direct import table contains no dynamic Visual C++ CRT import. |
| Actual version process | PASS | `ArchRescue 0.0.1 windows x86_64 build=64bf3a6c92702368a0cb4fe8dc66b9385536d3a4`, exit 0 |
| Native functional CLI and GUI | See individual results | [Windows native tests](WINDOWS-NATIVE-TESTS.md) |

Rust and Cargo 1.99.1 built in the Visual Studio 2022 x64 shell, with MSVC 14.44.35207 and SDK 10.0.26100.0. The lockfile SHA-256 was `c8aa495d554a8c719ed990afced04af614da5ae66c1108fd3781fac905bb7874`. The repository's `+crt-static` setting was retained.

The guest artifact is under the clean checkout at `target/x86_64-pc-windows-msvc/release/archrescue-boot-shell.exe`. The Linux host copy is `artifacts/windows/64bf3a6/archrescue-boot-shell.exe`, 769,536 bytes, SHA-256 `2c292f8f55c6f4d2ec0358d39eba365a91472082e4c7b72c4d5f3a52f3753ca6`.

Direct imports are Kernel32, Gdi32, User32, Ntdll, SetupAPI, BcryptPrimitives and the API set `api-ms-win-core-synch-l1-2-0.dll`. Presence and loader behavior under WinPE are tracked separately.

The first standalone compiler probe attempted to link only a static archive and hit LNK1561 because the entry point object was not selected. The repeat invoked the library manager to create the archive and supplied the probe object explicitly to the linker. The executable and Rust release build then ran successfully.

The initial clean `main` build at `b79257e3132c77ad54b2ccc924fcec4ded1a6ee3` also linked and ran. Its independent host copy is `artifacts/windows/b79257e/archrescue-boot-shell.exe`, SHA-256 `a2e2924ce43fdacc0974719225e1f96443c4c60917d0bab3d8768d5fefcc8ec1`. Two focused commits followed for BCD isolation and reentrant window teardown. Release gates were repeated in a clean checkout for each source revision.

Local raw evidence under `.agent-tmp/w10val/` includes `bundle-verify.log`, `source-checkout.log`, `native-build-probe-first.log`, `native-build.log` and `native-pe-{headers,imports,dependents}.txt`. Evidence for the intermediate BCD fix is under `.agent-tmp/w10val/final/`. The final release build evidence is under `.agent-tmp/w10val/gui-fixed/`. The host copy passed an independent PE-format and SHA-256 check.
