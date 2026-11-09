# P0 architecture

Version 0.0.1. Rust core with a native Win32 shell; no browser, WPF, network service, repair, formatting, installation, or migration execution.

Modules: core (typed errors, operation states), storage (read-only platform inventory and identity), windows-detection (offline filesystem evidence), diagnostics (EFI/BCD inspection), backup (safe browsing and verified selective copy), evidence (local report export), boot-shell (pt-BR GUI and diagnostic CLI).

Only backup and explicitly requested report export write user-selected destinations. Source files are opened read-only. Source and destination must have different filesystem identities and, on Windows, disjoint physical disk extents. Unknown identities fail closed. No overwrite policy: existing targets fail. Symlinks, reparse points, encrypted files, special files, and path traversal are unsupported. Operation states are PENDING, VALIDATING, RUNNING, VERIFYING, COMPLETED, FAILED, CANCELLED, BLOCKED. Completion requires rereading destination bytes and matching SHA-256, with source stability checks.

The supported operational assumption is an offline, quiescent filesystem with no concurrent adversarial path mutation. OS/filesystem metadata writes caused by mounting or reading are not equivalent to hardware write blocking; use read-only source virtual disks in acceptance tests. The application does not change disk attributes or mount unmounted partitions.

Reports are local and opt-in. They omit disk serial numbers and personal file contents; selected paths in backup manifests are necessary recovery metadata and require technician care. Reports on WinPE RAM disk do not survive reboot.

## Preserve & Rebuild boundary

Future interfaces: ProfileInventory, PreservationConstraints, VerifiedBackupReference, DeploymentPlan, AccountMapping, RestoreVerification. No P0 executable invokes Windows Setup, USMT, BCD repair, registry writes, formatting, or partition editing. Deployment must require a separately verified preservation plan and explicit authorization. Hard links are not independent backup and cannot survive formatting; Windows.old depends on a compatible non-formatting setup path. Program Files is not universally migratable by copying. Never write concurrently into Windows Setup's target tree. Restore completion must follow file/hash and account mapping verification.
