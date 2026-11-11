# ArchRescue environment hygiene incident

Measured on 2026-11-10 in America/Sao_Paulo. The incident interrupted the M1/M2/M3 wave on `codex/m1-m3-recovery-advancement` at `ae04e7d`. The tracked tree was clean at the start.

## Measured results

All values are bytes. Host directory sizes use allocated blocks as reported by `du`. Filesystem free space uses `df` on the host and `Win32_LogicalDisk.FreeSpace` in the guest.

| Metric | Before | After | Reclaimed |
|---|---:|---:|---:|
| Project directory | 113,198,657,536 | 31,239,278,592 | 81,959,378,944 |
| Repository .agent-tmp | 110,752,718,848 | 29,625,659,392 | 81,127,059,456 |
| Repository Cargo output | 1,237,168,128 | 0 | 1,237,168,128 |
| Retained artifacts | 1,206,198,272 | 1,610,870,784 | -404,672,512 |
| Host filesystem free space | 264,202,801,152 | 310,274,813,952 | 46,072,012,800 |
| W10Val C: free space | 5,264,203,776 | 12,405,846,016 | 7,141,642,240 |

The project directory fell from 105.4 GiB to 29.1 GiB. The host filesystem had 42.9 GiB more free space at final measurement. Same-filesystem relocation accounts for some of the repository reduction and contributes zero to filesystem capacity recovery. Compression and shared Btrfs extents also affect the relationship between per-path accounting and filesystem free space. Retained artifacts grew when the matching guest EXE, ISO and compact QA evidence were promoted. Git tracked 552,443 bytes at the start, with 1.57 MiB of loose objects.

W10Val rose from 4.9 GiB to 11.9 GiB immediately after bulk cleanup and measured 11.6 GiB after the final native probes and their cleanup. The 15 GiB floor still blocks substantial new Windows builds. A WinPE build already in progress when the incident arrived finished with exit 0 and no mounted WIMs. The new ISO and executable were promoted to local retention after SHA-256 verification. The recovered host capacity supported QA with that ISO. Guest disk capacity was held at the existing size.

## Ownership and preservation

The Linux inventory at `.agent-tmp/hygiene-audit/inventory.md` records allocation, logical size, modification time, origin, dependencies and action. The same directory holds the exact host and Windows action logs, filesystem and guest snapshots, SHA-256 records and VirtualBox media inspections. Historical acceptance reports and source remain available. Compact prior logs, sampled exports, build metadata, hashes and screenshots remain in their original evidence directories. The synthesized 512 MiB payload and the separately recovered partial prefix were hashed before retiring duplicate exports.

Linux file removals were limited to known test outputs under the project scratch tree with matching path, device, inode, size and modification time. Open-file checks were run before removal. Four detached VirtualBox readbacks with no usage or child media were deleted by their verified VirtualBox UUIDs. The read-only QEMU source duplicate was byte-identical to the retained `w10val/qa/qa-source.vdi` base. The base is the provenance for reproducing the independent readback images and the new disposable repair clone. The original Windows installation media in the operator's Downloads directory is retained.

The Windows plan required a quiet native build and servicing environment, a clean WIM mount inventory and exclusive read probes before removal. Four locally retained ISO hashes matched the corresponding guest artifacts. The earliest superseded guest-only P0 ISO retains its guest hash and build metadata. The cleanup kept the installed toolchains, profile data, guest source checkouts, registry/cache dependencies and compact logs. A historical abandoned WIM extraction containing reparse points was excluded by the path check and remains in the older corrective checkout. The cleanup also left unnamed Windows temp files whose ownership was uncertain.

### Preserved large artifacts and dependencies

- Active W10Val boot disk, snapshots and attached fixture parents under the VirtualBox and `.agent-tmp/w10val/disks` paths. The QEMU host audit did not modify any W10Val disk image.
- ArchRescue-QA configuration, snapshot, source and destination parents under `.agent-tmp/w10val/qa` and the attached source and destination of ArchRescue-P0-Corrective-QA under `.agent-tmp/p0-corrective/virtualbox`. Their exact paths, medium IDs, parent UUIDs and attachments are in the before/after VirtualBox logs.
- Local WinPE ISOs under `artifacts/windows/9e624d2`, `64bf3a6`, `513c8de` and `ae04e7d`. The three P0 variants have a registered VM or snapshot dependency. The wave build retains its exact EXE, ISO, import list and build log.
- The active Rust cache moved outside the repository and accounts for about 1.3 GiB after the test run. The wave QA evidence was promoted to `artifacts/evidence/m1-m3`.
- The repair clone, its QCOW2 overlay, the disposable destination and extracted QEMU runtime were retired after the guest was stopped and the compact evidence promoted. The external run accounted for 10,938,540,032 allocated bytes immediately before closure. The measured filesystem gained 6,900,797,440 available bytes during that closure interval. The run manifest and symlink-unlink log enumerate exact removals. The underlying VirtualBox media were preserved.

The before/after VirtualBox snapshot names, UUIDs, VM power states and attached medium UUIDs matched. W10Val was running during the incident checks and the three historical QA VMs were powered off. SSH returned `w10val\archrescue`. MSVC 19.44.35229, Rust 1.99.1, SDK 10.0.26100.0, ADK DISM 10.0.26100.9457 and the amd64 WinPE image and media scripts remained available. DISM reported no mounted images after the cleanup.

## Guardrails

The [workspace storage policy](../WORKSPACE-STORAGE.md) documents the configurable external scratch root, ownership manifests, capacity checks, one active heavy run per scratch root, cleanup and retention. Linux QA scripts check disk capacity, and Rust tests use an external Cargo cache by default. The WinPE build now checks free space, records its dedicated staging and Cargo paths, and cleans successful staging after artifact promotion and WIM-state checks. Native PowerShell parsing, the actual low-space refusal and a bounded scratch inventory passed. An end-to-end build of the new guardrails remains pending while W10Val is below the 15 GiB floor. Linux scratch tests exercised insufficient capacity, concurrent-run refusal, symlink rejection and idempotent cleanup.

The compact audit archive is under `artifacts/evidence/hygiene-2026-11-10`. `wave-cleanup-manifest.json` records the external run closure, and `wave-runtime-links.tsv` records links unlinked without traversal. `windows-wave-closure.log` records six individually verified removals from the later probe staging. The final toolchain/SSH probes and VM mapping checks are preserved there.

## Exact cleanup record

Paths below are from the verified action logs. Windows sizes are logical file bytes. Host sizes are accounted file allocations before deletion.

<details>
<summary>Host removals</summary>

| Removed path | Bytes | Classification |
|---|---:|---|
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/integrity/source-before.raw` | 9,611,247,616 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/integrity/source-before.ntfs` | 9,595,518,976 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/virtualbox/readback-pre-reboot/disk.raw` | 55,574,528 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/virtualbox/readback-pre-reboot/partition.ntfs` | 83,755,008 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/virtualbox/readback-post-reboot/disk.raw` | 55,574,528 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/virtualbox/readback-post-reboot/partition.ntfs` | 83,755,008 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/virtualbox/source-after/disk.raw` | 9,612,296,192 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/virtualbox/source-after/partition.ntfs` | 9,599,713,280 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/readback-pre-reboot/disk.raw` | 58,720,256 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/readback-pre-reboot/partition.ntfs` | 92,143,616 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/readback-pre-reboot-verified/disk.raw` | 58,720,256 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/readback-pre-reboot-verified/partition.ntfs` | 92,143,616 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/source-after/disk.raw` | 9,611,247,616 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/source-after/partition.ntfs` | 9,595,518,976 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/readback-post-reboot/disk.raw` | 58,720,256 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/readback-post-reboot/partition.ntfs` | 92,143,616 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/gui/progress-readback/disk.raw` | 59,768,832 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/gui/progress-readback/partition.ntfs` | 96,337,920 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/destination-failure/readback/disk.raw` | 59,768,832 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/destination-failure/readback/partition.ntfs` | 96,337,920 | Readback image |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/virtualbox/source-after/export/gui-cancellation-final/payload-512m.bin` | 536,870,912 | Synthetic payload |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/source-after/export/gui-cancellation-final/payload-512m.bin` | 536,870,912 | Synthetic payload |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/gui/progress-readback/payload-512m.bin` | 536,870,912 | Synthetic payload |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/destination-failure/export/payload-512m.bin` | 176,291,840 | Synthetic payload |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/gui/progress-dest.vdi` | 607,141,888 | Unregistered QA destination |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/destination-failure/loss-dest.vdi` | 269,500,416 | Unregistered QA destination |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/source.vdi` | 9,611,350,016 | Duplicate source VDI |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/style-baseline-tree.5BjLIL1P/target` | 94,130,176 | Obsolete Cargo output |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/final-audit/mutation-target` | 148,819,968 | Obsolete Cargo output |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/w10val/qa/qa-readback.vhd` | 20,987,904 | VirtualBox detached readback 060ab6d0-71d7-402f-b424-0752c8dd677e |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/w10val/qa/qa-final-readback.vhd` | 23,085,056 | VirtualBox detached readback 1683f8ae-a777-4613-ba46-b20b068c8ff0 |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/w10val/qa/qa-post-reboot-readback.vhd` | 23,085,056 | VirtualBox detached readback 322747bd-105c-4eaf-84f3-afba5b0ceda1 |
| `/home/cainhooow/Projetos/archrescue/.agent-tmp/p0-corrective/qemu-ovmf/destination.vdi` | 78,659,584 | VirtualBox detached readback 936077b9-176e-45e5-a7d8-5538b00c6896 |

</details>

<details>
<summary>W10Val removals</summary>

| Removed path | Bytes | Classification |
|---|---:|---|
| `C:\Users\archrescue\ArchRescueBuild-b79257e\source\target` | 477,161,392 | Superseded P0 Cargo cache |
| `C:\Users\archrescue\ArchRescueBuild-9e624d2\source\target` | 478,702,701 | Superseded P0 Cargo cache |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\source\target` | 478,700,834 | Superseded P0 Cargo cache |
| `C:\Users\archrescue\ArchRescueP0Corrective-Baseline\source\target` | 291,034,644 | Superseded corrective Cargo cache |
| `C:\Users\archrescue\ArchRescueP0Corrective-Candidate\source\target` | 292,142,364 | Superseded corrective Cargo cache |
| `C:\Users\archrescue\ArchRescueP0Corrective-Final\source\target` | 480,082,346 | Verified 513c8de EXE and ISO promoted to host |
| `C:\Users\archrescue\ArchRescueWave-Foundation\source\target` | 358,466,019 | Foundation cache superseded by the wave build |
| `C:\Users\archrescue\ArchRescueWave\source\target` | 621,124,492 | Current build outputs promoted and matched to host hashes |
| `C:\Users\archrescue\ArchRescueBuild-b79257e\installers` | 88,349,897 | Provisioning downloads, installed toolchain directories preserved |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\qa-readback.vhd` | 20,984,832 | Detached guest-local prior QA readback; host evidence preserved |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\qa-final-readback.vhd` | 23,082,496 | Detached guest-local prior QA readback; host evidence preserved |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\qa-post-reboot-readback.vhd` | 23,082,496 | Detached guest-local prior QA readback; host evidence preserved |
| `C:\Users\archrescue\ArchRescueBuild-b79257e\source\.agent-tmp\winpe-9fc74257107b48edb1fde87f7b8ca0e2\bootbins` | 11,415,080 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-b79257e\source\.agent-tmp\winpe-9fc74257107b48edb1fde87f7b8ca0e2\media` | 396,715,987 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-b79257e\source\.agent-tmp\winpe-9fc74257107b48edb1fde87f7b8ca0e2\mount` | 0 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-9e624d2\source\.agent-tmp\winpe-0d0f47908aac4f1993a69593f2a1d0af\bootbins` | 11,415,080 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-9e624d2\source\.agent-tmp\winpe-0d0f47908aac4f1993a69593f2a1d0af\media` | 396,723,003 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-9e624d2\source\.agent-tmp\winpe-0d0f47908aac4f1993a69593f2a1d0af\mount` | 0 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\source\.agent-tmp\winpe-88bee7b410614c5fb8757d1d78db5088\bootbins` | 11,415,080 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\source\.agent-tmp\winpe-88bee7b410614c5fb8757d1d78db5088\media` | 396,722,951 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\source\.agent-tmp\winpe-88bee7b410614c5fb8757d1d78db5088\mount` | 0 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueP0Corrective-Final\source\.agent-tmp\winpe-7aa120ebd77040abb5e8dd3d6f6d0464\bootbins` | 0 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueP0Corrective-Final\source\.agent-tmp\winpe-7aa120ebd77040abb5e8dd3d6f6d0464\media` | 392,528,659 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueP0Corrective-Final\source\.agent-tmp\winpe-ef8c728a4e704c73b329df3724eb73d8\bootbins` | 11,415,080 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueP0Corrective-Final\source\.agent-tmp\winpe-ef8c728a4e704c73b329df3724eb73d8\media` | 396,722,953 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueP0Corrective-Final\source\.agent-tmp\winpe-ef8c728a4e704c73b329df3724eb73d8\mount` | 0 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueWave\source\.agent-tmp\winpe-6152a4e2d4c84c90a2887ac3da1f0209\bootbins` | 11,415,080 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueWave\source\.agent-tmp\winpe-6152a4e2d4c84c90a2887ac3da1f0209\media` | 396,975,867 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueWave\source\.agent-tmp\winpe-6152a4e2d4c84c90a2887ac3da1f0209\mount` | 0 | Generated WinPE staging; no WIM currently mounted |
| `C:\Users\archrescue\ArchRescueBuild-b79257e\source\artifacts\winpe-9fc74257107b48edb1fde87f7b8ca0e2\archrescue-0.0.1-x64.iso` | 400,224,256 | Guest ISO staging copy; hash and metadata retained |
| `C:\Users\archrescue\ArchRescueBuild-9e624d2\source\artifacts\winpe-0d0f47908aac4f1993a69593f2a1d0af\archrescue-0.0.1-x64.iso` | 400,230,400 | Guest ISO staging copy; hash and metadata retained |
| `C:\Users\archrescue\ArchRescueBuild-64bf3a6\source\artifacts\winpe-88bee7b410614c5fb8757d1d78db5088\archrescue-0.0.1-x64.iso` | 400,230,400 | Guest ISO staging copy; hash and metadata retained |
| `C:\Users\archrescue\ArchRescueP0Corrective-Final\source\artifacts\winpe-ef8c728a4e704c73b329df3724eb73d8\archrescue-0.0.1-x64.iso` | 400,230,400 | Guest ISO staging copy; hash and metadata retained |
| `C:\Users\archrescue\ArchRescueWave\source\artifacts\winpe-6152a4e2d4c84c90a2887ac3da1f0209\archrescue-0.0.1-x64.iso` | 400,484,352 | Guest ISO staging copy; hash and metadata retained |

</details>

## Verdicts

```text
PROJECT WORKSPACE HYGIENE: PASS
TEMPORARY STORAGE POLICY: PASS
HOST DISK SPACE: PASS
W10VAL FREE SPACE: BLOCKED — 11.6 GiB free, 15 GiB minimum
WINDOWS TOOLCHAIN INTEGRITY: PASS
VIRTUAL MACHINE INTEGRITY: PASS — observed media mappings
RECOVERY SNAPSHOT INTEGRITY: PASS — snapshot UUID and chain inventory
EVIDENCE PRESERVATION: PASS
PERMANENT CLEANUP AUTOMATION: PASS — host cleanup and Windows guard smoke tests
DEVELOPMENT RESUMPTION READINESS: PASS — host QA with the promoted ISO
```
