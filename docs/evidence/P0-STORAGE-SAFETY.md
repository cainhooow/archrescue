# P0 storage and backup safety evidence

The executed tests use ordinary disposable fixture files, not raw disks. Source filesystem: `unix-dev:50` (workspace). Destination: `unix-dev:33` (tmpfs under `/dev/shm/archrescue-tests/.agent-tmp`). These are **not Windows virtual disk identities** and do not prove physical-disk isolation. The production Windows path queries volume GUIDs and physical disk extents and rejects unknown or intersecting extents; runtime validation remains BLOCKED.

## Independent executable check

Command:

```sh
ARCHRESCUE_TEST_DESTINATION=/dev/shm/archrescue-tests/.agent-tmp python3 scripts/cli_acceptance.py
```

The script requires an executable build ID matching the expected source commit. It uses Python hashlib, file comparisons and an independent restoration copy to check the Rust executable's output. [Raw summary](raw/cli-summary.json), [completed operation](raw/backup-completed.json), [conflict result](raw/backup-conflict.json).

- Dataset: three files, 262,174 bytes; nested directory, Unicode name, empty file and empty directory.
- Source manifest before and after: identical.
- Destination hashes/content: identical to source.
- Independent restored copy: identical to source.
- First operation: COMPLETED, all three files verified.
- Repeated copy into existing targets: FAILED/CONFLICT, preserving both source and existing destination.
- Report JSON/text export: read back successfully.
- Fixture cleanup: only uniquely created fixture directories removed after assertions. No physical disk or partition modified.

| File | Bytes | Independently measured SHA-256 |
|---|---:|---|
| `perfil/Documentos/dados.bin` | 262144 | `2312394bd99545d9de131c24efb781e765ac1aec243f2ed9347597a793a415e9` |
| `perfil/olá.txt` | 30 | `94a89a51c713a7d1249880ca01243bdc1aee676a00cb927e995d39ed038d92bd` |
| `perfil/zero.txt` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

## Controls and limits

Preflight is read-only and checks absolute roots, traversal, source/destination overlap, identity, free bytes, unreadable entries, conflicts, special files and ambiguous Windows names. Copy uses create-new destination files and read-only source files. Short writes are counted, cancellation is checked between chunks and during hashing, partial results retain incomplete state, destination files are flushed, and both destination and source are reread for SHA-256 verification. Reports never label incomplete operations successful.

All symlink/reparse components are rejected in supported path validation. This is not protection against an adversary concurrently replacing path components between checks and opens; the supported source must be offline and quiescent. Windows share modes, open-by-handle identity binding and race-hardened traversal require further qualification. Source files are never intentionally written by production copy code, but OS mount/read metadata writes are not prevented. Use hypervisor read-only source attachment for VM acceptance and compare source image hashes.

Content-only copying does not preserve ACLs, EFS, alternate streams, timestamps or sparse allocation. Those limitations are shown in result objects and the GUI confirmation. BitLocker volumes are not unlocked and EFS files are rejected by the Windows metadata guard. No privilege-bypass code exists.

An interrupted process can leave a partial destination without a persisted manifest. Cooperative GUI cancellation produces a report in memory; export before closing or rebooting. CLI callers must retain structured stdout. No resumable backup or power-loss recovery is claimed. The destination tmpfs used for this test is volatile; persistent backup and post-reboot verification remain VM acceptance requirements.
