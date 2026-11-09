# P0 build results

Date: 2026-11-09. Final source commit: `32c1eb3e6a14f1f1d7c1295d732c7a0cd5bd27c0`.

| Check | Result | Actual evidence |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | First gate in `scripts/test.sh`; the script continued through tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS | [Rust gates](raw/rust-gates.txt) |
| `cargo test --workspace --locked` | PASS | 31 tests; 0 failures, 0 ignored; [Rust gates](raw/rust-gates.txt) |
| `cargo clippy --workspace --all-targets --target x86_64-pc-windows-msvc --locked -- -D warnings` | PASS | [Windows target lint/type checking](raw/windows-clippy.txt); no Windows executable run |
| Linux release build, locked dependencies, source commit embedded | PASS | [Release output](raw/linux-release-build.txt) |
| Independent CLI acceptance, matching embedded build ID | PASS | [Summary](raw/cli-summary.json), [full output](raw/cli-acceptance.txt) |
| Windows MSVC executable build | BLOCKED | [Actual attempt](raw/windows-build.txt): `link.exe` not found; exit 101 |
| WinPE ISO build / import-DLL inspection | BLOCKED | No Windows host, linker/SDK, ADK or WinPE add-on |
| Native GUI visual/interaction validation | BLOCKED | No runnable Windows/WinPE environment |
| PowerShell build-script execution | BLOCKED | No PowerShell/Windows deployment tools; script is prepared, not execution-validated |

Executed test destination: `/dev/shm/archrescue-tests/.agent-tmp`, a different real filesystem from the workspace. No Windows test result is inferred from Linux fixture success. The cross-target lint check covers Windows-only API declarations and shell source, not ABI behavior at runtime.

Executable: `target/release/archrescue-boot-shell` (Linux x86_64). SHA-256:

```text
b78851d3a557f239f4a7a3ad87396c670dbcf257b9b4669a151bc9fa228256a6
```

Version output:

```text
ArchRescue 0.0.1 linux x86_64 build=32c1eb3e6a14f1f1d7c1295d732c7a0cd5bd27c0
```

Toolchain versions and installed targets: [final-toolchains.txt](raw/final-toolchains.txt). Locked dependency versions/licenses: [dependency inventory](../decisions/THIRD-PARTY-LICENSES.md). Build outputs are ignored; the local source kit is generated under `artifacts/` after the evidence commit.

## Corrections during execution

Initial Windows compilation exposed a SetupAPI handle type mismatch and a missing input-control feature import; both were corrected. Lint found unused state and test syntax issues; these were corrected without relaxing warnings. Review replaced raw boolean-containing structure reads with bounded scalar parsing and guarded native callback reentrancy. A report build-ID field was initially omitted from its constructor, causing a later build failure; the previous binary still ran. The final constructor is fixed and CLI acceptance now requires the expected embedded commit. The final evidence above supersedes that earlier stale-binary run; no acceptance credit is assigned to it for the final source.
