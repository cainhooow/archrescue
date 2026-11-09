# ArchRescue P0 engineering handoff

## A. Execution Summary

Initial repository HEAD: none; the directory was not a Git repository and contained only `ARCHRESCUE_ROADMAP.md`. The initial roadmap baseline is commit `161b00a`. Branch: `p0/first-prototype`. Host: Arch Linux x86_64, kernel 7.2.14-arch1-1, Rust/Cargo 1.99.1.

Final executable source HEAD: `32c1eb3e6a14f1f1d7c1295d732c7a0cd5bd27c0`. The final repository HEAD is the subsequent documentation/evidence commit containing this handoff; its exact resolved hash and all commit hashes are recorded in the local `artifacts/DELIVERY.json` and `artifacts/git-audit.txt` after committing. This distinction prevents pretending a document contains its own commit hash.

Implemented a real Rust vertical slice with Windows storage adapters, native pt-BR shell source, offline candidate/boot evidence, file browsing, verified selective content copy and report export. Built and executed the Linux CLI. Windows linking, WinPE ISO generation and VM runtime remain blocked; the bootable milestone is not complete.

## B. Deliverables

- `crates/core`: structured errors, operation state machine, version/build identity.
- `crates/storage`: read-only Windows disk/volume queries, bounded layout parser, identity/free-space validation; Linux filesystem primitives for fixture execution.
- `crates/windows-detection`: filesystem/registry-signature/kernel-PE corroboration, architecture and explicit limitations; no invented version.
- `crates/diagnostics`: EFI/BCD path/header evidence and read-only Windows bcdedit enumeration; no repair.
- `crates/backup`: explorer and selective, conflict-rejecting, cancellable content copy with SHA-256 source/destination verification.
- `crates/evidence`: structured and readable opt-in local report export with readback verification.
- `apps/boot-shell`: actual portable CLI and native Windows GUI source with seven pt-BR sections, worker threads and console access.
- Executable: `target/release/archrescue-boot-shell`; copied to `artifacts/archrescue-0.0.1-linux-x86_64` for handoff.
- Local source kit: `artifacts/archrescue-0.0.1-local-build-kit.tar.gz`; no Windows binaries or ISO inside.
- Build scripts: `packaging/winpe/build.ps1`, `startnet.cmd`, `unattend.xml`; test scripts: `scripts/test.sh`, `scripts/cli_acceptance.py`.
- Fixtures: executable generators in `crates/backup/src/tests.rs`, `crates/storage/src/layout.rs`, `crates/evidence/tests/contracts.rs` and the Python CLI checker. Disposable runtime data is generated under `.agent-tmp/` and the explicitly configured separate filesystem.
- Documentation: [architecture](architecture/P0-ARCHITECTURE.md), [distribution ADR](decisions/ADR-WINPE-DISTRIBUTION.md), [environment](evidence/P0-ENVIRONMENT-AUDIT.md), [build](evidence/P0-BUILD-RESULTS.md), [boot](evidence/P0-BOOT-EVIDENCE.md), [storage](evidence/P0-STORAGE-SAFETY.md), [test matrix](qa/P0-TEST-MATRIX.md), [negative tests](qa/P0-NEGATIVE-TESTS.md), [homologation](qa/P0-FINAL-HOMOLOGATION.md), [dependency licenses](decisions/THIRD-PARTY-LICENSES.md), and README.

No Windows executable or ISO exists. Artifact checksums are recorded in `artifacts/SHA256SUMS` and the delivery manifest.

## C. Feature Acceptance Matrix

The [full matrix](qa/P0-TEST-MATRIX.md) gives expected behavior, actual observations, commands, evidence and limitations for every capability. All ten target feature acceptance rows are BLOCKED because the mandatory Windows/WinPE environment is unavailable. This does not erase the executed portable subtest results:

| Capability | Target acceptance | Executed evidence |
|---|---|---|
| P0-F01 | BLOCKED | Actual MSVC build fails for missing linker; no ISO/boot |
| P0-F02 | BLOCKED | Windows shell type-checks; GUI not executed |
| P0-F03 | BLOCKED | Windows adapter type-checks; actual API enumeration untested |
| P0-F04 | BLOCKED | GPT/EFI malformed-response parser tests PASS |
| P0-F05 | BLOCKED | Candidate, invalid-folder, invalid-PE/hive and empty-tree tests PASS |
| P0-F06 | BLOCKED | Actual Linux filesystem space checks and error contracts PASS |
| P0-F07 | BLOCKED | Missing/malformed/unreadable boot-file tests PASS; bcdedit unexecuted |
| P0-F08 | BLOCKED | Actual file listing and metadata tests PASS; GUI flow unexecuted |
| P0-F09 | BLOCKED | Independent real-file copy/restore verification PASS; Windows/VM tests blocked |
| P0-F10 | BLOCKED | Local JSON/text roundtrip PASS; WinPE persistence untested |

## D. Boot Verification

Intended target: x64 UEFI. No QEMU/OVMF, VirtualBox, Hyper-V or equivalent run occurred. No firmware configuration, ISO identity/hash, application startup result or Secure Boot evidence exists. The Windows build attempt is recorded in `docs/evidence/raw/windows-build.txt`; it stops at missing `link.exe`. The kit and [VM procedure](../tests/vm/ACCEPTANCE.md) are prepared for a qualified Windows build host and two disposable VM environments.

## E. Storage and Backup Verification

Actual identities: `unix-dev:50` source and `unix-dev:33` destination. These are Linux filesystem identities, not Windows virtual-disk identities. Dataset: three files totaling 262174 bytes, nested directories, Unicode name, empty file and empty directory. Python hashlib independently verifies that source-before, source-after, destination and restored-copy manifests match. SHA-256 values and operation IDs are in [CLI summary](evidence/raw/cli-summary.json). The suite exercises all twelve requested negative categories with explicitly scoped fixtures; actual detach/ACL/driver/Windows scenarios remain blocked. No host raw disk or partition was modified.

## F. Security and Licensing

No destructive storage paths, source write handles, overwrite policy, encryption bypass, network telemetry, remote publication or bundled proprietary binaries. Unknown destination identity fails closed. Sources must be offline and quiescent; filesystem path race hardening, hardware write blocking and abrupt-power-loss recovery are not provided. Content-only copying omits ACLs, timestamps, alternate streams, sparse allocation and encrypted-file support. Reports can contain necessary paths/volume identities but no personal file contents or recovery keys.

The final source license is unresolved. Exact ADK/WinPE redistribution rights have not been established; public ISO distribution is blocked. Twenty-three locked third-party source dependencies have declared-license records. See the distribution ADR and homologation risks.

## G. Git Audit

Local commits before the final evidence commit:

1. `161b00a` — original roadmap, environment audit and P0 boundaries.
2. `29de404` — Rust diagnostic/backup vertical slice, native shell and tests.
3. `88c78b8` — local WinPE ISO kit and VM acceptance procedure.
4. `d925948` — bounded metadata parsing, callback reentrancy and regression coverage.
5. `32c1eb3` — corrected build identity, stale-executable guard and clearer presentation.
6. Final evidence commit — this handoff, raw logs, test matrix, licensing inventory and roadmap addendum; exact hash in delivery manifest and `git log`.

No remote was configured. No push, deployment, release, pull request, ClickUp modification or external message occurred. The original roadmap bytes are preserved as the prefix of the current file, followed by an English evidence addendum. Local author identity is explicitly `ArchRescue Engineering <engineering@archrescue.invalid>`; no user identity was impersonated.

## H. Remaining Work

1. Provision Windows x64 with MSVC/SDK and legitimately obtained matching ADK/WinPE; execute the build kit and inspect real imports/API-set compatibility.
2. Boot the ISO on QEMU/OVMF and Hyper-V or VirtualBox with disposable Windows/source and backup virtual disks. Exercise all ten features, GUI/keyboard behavior, fallback, source image hashes, report persistence and restored files after reboot.
3. Test Secure Boot against documented firmware/trust configuration. Record evidence matched to ISO/executable hashes.
4. Complete Windows ACL/EFS/inaccessible-volume and real detach/error testing. Harden path-race behavior if support expands beyond offline quiescent sources, and qualify BCD query behavior/timeouts.
5. Resolve localization/accessibility gaps, candidate version metadata and BCD reference-consistency limitations against the final P0 scope.
6. Obtain independent QA sign-off, choose the source license and verify distribution terms before any public release.

There are no failing tests in the final executed portable suite. Windows build and operational acceptance are blocked; GUI, WinPE script execution and relevant platform behavior have no execution evidence. Earlier compilation/lint issues were corrected and disclosed in build results.

## I. Final Homologation Verdict

**P0 BLOCKED**
