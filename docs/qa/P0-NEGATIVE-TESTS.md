# P0 negative test results

The matrix below records the historical run. The [corrective regression](P0-CORRECTIVE-HOMOLOGATION.md#c-native-windows-failure-reproduction-and-correction) replaces the in-flight rename fixture and reports live media removal separately.

All PASS entries below are executed controlled-fixture tests from [Rust gate output](../evidence/raw/rust-gates.txt), not Windows VM acceptance. Run `scripts/test.sh` with an explicit different-filesystem test destination. Tests fail if that prerequisite is absent; they are not silently skipped.

| Required scenario | Executed test / observation | Result | Scope limitation |
|---|---|---|---|
| 1. Same volume / physical storage | `same_volume_rejected_before_writes`; `shared_extent_rejected` | PASS | Real same-filesystem rejection; shared Windows extents use an explicitly synthetic contract; Windows API test BLOCKED |
| 2. Destination beneath source | `nested_destination_rejected` | PASS | No new nested copy and original content retained |
| 3. Insufficient free space | `insufficient_space_uses_actual_free_space` | PASS | Sparse source length exceeds measured destination availability; no destination fill or raw disk operation |
| 4. Selected file unreadable | `unreadable_source_is_not_silently_skipped` | PASS | Unix permission denial with permission restored for fixture cleanup; Windows ACL/EFS test BLOCKED |
| 5. Failure partway through | `partway_conflict_reports_partial_result` | PASS | Actual destination create-new failure injected after first file; source bytes unchanged |
| 6. Malformed filesystem/storage metadata | `malformed_space_rejected`; layout truncation/bounds/count tests | PASS | Controlled byte-response parser and contract fixtures; damaged Windows volume test BLOCKED |
| 7. No Windows found | `no_windows_is_an_empty_result` | PASS | Actual empty directory tree; no installation objects fabricated |
| 8. Invalid Windows directory | `directory_named_windows_is_not_an_installation`; malformed PE and hive tests | PASS | Filesystem marker validation; real Windows image test BLOCKED |
| 9. Boot configuration unreadable | `unreadable_boot_configuration_is_reported`; malformed/missing BCD | PASS | File permission/header checks; Windows bcdedit execution BLOCKED |
| 10. Cancellation | `cancellation_reports_exact_partial_bytes`; preflight cancellation | PASS | 65,536 bytes retained from 200,000 planned; CANCELLED, not verified; source unchanged |
| 11. Destination already exists | `existing_destination_never_overwritten`; CLI repeated copy | PASS | Existing bytes and source retained |
| 12. Volume unavailable mid-operation | `unavailable_destination_never_completes` | PASS | Controlled destination path renamed away during copy; actual unplug/VM detach BLOCKED |

Additional regression tests: destination hash corruption; symlink source/destination ancestors; overlapping and case-colliding selections; unsafe portable names; invalid operation state transitions; malformed driver boolean bytes never reinterpreted as Rust bools; report roundtrip; positive copy/empty-directory preservation.

Negative tests deliberately inject faults only into owned fixture paths. Their assertions include source preservation where applicable. Missing hardware, real Windows data, destructive tests and VM scenarios are not converted into PASS results by these fixtures.
