# Workspace storage policy

Keep source, tests, documentation, compact evidence and selected build artifacts in the checkout. Treat `.agent-tmp/` as scratch for small run logs and fixtures. Keep heavy scratch under a dedicated operator-owned directory outside the checkout. An external path on the same filesystem reduces repository allocation only.

## Linux builds and VM tests

`scripts/test.sh` checks available space and defaults Cargo output to `$XDG_CACHE_HOME/archrescue/cargo-target` or `$HOME/.cache/archrescue/cargo-target`. `ARCHRESCUE_SCRATCH_ROOT` selects a different dedicated root. `CARGO_TARGET_DIR` can select an existing active cache.

Before a large job, account for the source and destination copies, largest possible writable overlay, extraction space, Cargo output and promoted evidence. For example, reserve 40 GiB of run space and leave 5 GiB available:

```bash
run=$(python3 scripts/scratch.py open --budget-bytes 42949672960 \
    --dependency '/absolute/path/to/the/read-only/source.vdi' \
    --retain '/absolute/path/to/promoted-evidence' 'UEFI recovery QA')
```

Create only disposable files under the returned path. The manifest records the run, ownership, directory identity, budget, external dependencies and expected retained outputs. The scratch root admits one active image-heavy run at a time. A deliberately retained run consumes space and should have an evidence or recovery reason recorded. Separate roots share any physical filesystem capacity underneath them.

After the job, record VM configuration and command output, extract only the required logs and evidence, independently check hashes, shut down guest processes, and confirm relevant VM and snapshot mappings. Then close the exact run:

```bash
python3 scripts/scratch.py close "$(basename "$run")" --cleanup
```

Cleanup verifies the registered directory identity, rejects links and cross-filesystem entries, checks for open files and VirtualBox media registrations, checks promoted evidence paths, and records the removed paths and accounted size. Use VirtualBox media operations for registered disposable media after proving it has no VM, child, or snapshot dependency. Record free space using the relevant filesystem before and after deletion. Path-level allocation and actual free-space changes can differ because of compression, snapshots, or shared extents.

## W10Val and WinPE

The existing `packaging/winpe/build.ps1` checks 15 GiB of free space on the Windows system volume and a staging budget with a free-space reserve. It uses a dedicated scratch root under `%LOCALAPPDATA%\ArchRescue\scratch\winpe` unless `ARCHRESCUE_WINPE_WORK_ROOT` identifies a different local fixed volume. Each build records its staging and Cargo paths, identity, budget, retained outputs and cleanup state in `scratch-manifest.json` next to the ISO. After promotion and WIM-state verification, successful builds remove owned staging and build output. A failed or explicitly retained run leaves its exact location and requires state-aware review.

After native tests, verify the source commit and promote the selected EXE, ISO, metadata and logs before deleting superseded Cargo targets or guest staging copies. Inspect the actual DISM mount list. Keep the installed SDK, ADK, WinPE add-on, MSVC, Rust, Git and SSH. Preserve guest disks and recovery snapshots. A volume below the 15 GiB floor blocks substantial new Windows builds. An already promoted ISO can still be tested on the host using disposable QA media.

## Retained evidence

Keep commit IDs, artifact hashes, build metadata, focused test logs, selected screenshots and independent readback results. Record fixture provenance and regeneration steps in the QA report. Keep large images only through their active validation or rollback lifetime, then remove the owned disposable copies once dependencies and evidence are verified. Existing VM media referenced by a live configuration or snapshot remains managed by its hypervisor.
