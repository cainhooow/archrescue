//! Controlled fixtures only. Cross-filesystem tests require an explicit disposable destination.

#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};

use super::*;

struct Fixture {
    root: PathBuf,
    destination: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = Operation::new().id;
        #[cfg(windows)]
        let base = PathBuf::from(
            std::env::var_os("ARCHRESCUE_TEST_SOURCE")
                .expect("Set ARCHRESCUE_TEST_SOURCE to a disposable persistent GPT volume"),
        );
        #[cfg(not(windows))]
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.agent-tmp/tests");
        fs::create_dir_all(&base).unwrap();

        let root = base.join(&id);
        fs::create_dir(&root).unwrap();

        let external = std::env::var_os("ARCHRESCUE_TEST_DESTINATION").expect(
            "Set ARCHRESCUE_TEST_DESTINATION to a disposable directory on a separate filesystem; \
             see scripts/test.sh",
        );
        let destination = PathBuf::from(external).join(&id);
        fs::create_dir(&destination).unwrap();

        assert_ne!(
            identity(&root).unwrap().volume,
            identity(&destination).unwrap().volume,
            "Tests require two real filesystems"
        );

        Self {
            root: fs::canonicalize(root).unwrap(),
            destination: fs::canonicalize(destination).unwrap(),
        }
    }

    fn put(&self, name: &str, data: &[u8]) {
        let path = self.root.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, data).unwrap();
    }

    fn copy(&self, selections: &[&str]) -> BackupResult {
        recover(
            &self.root,
            &selections.iter().map(PathBuf::from).collect::<Vec<_>>(),
            &self.destination,
            &AtomicBool::new(false),
            |_, _| {},
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        let _ = fs::remove_dir_all(&self.destination);
    }
}

#[test]
fn portable_paths_reject_traversal_and_devices() {
    for path in [
        "../secret",
        "/absolute",
        "NUL",
        "dir/CON.txt",
        "name:stream",
        "trailing.",
    ] {
        assert!(portable_relative(Path::new(path)).is_err(), "{path}");
    }
}

#[test]
fn verified_copy_preserves_hierarchy_empty_dirs_and_source() {
    let fixture = Fixture::new();
    let data: Vec<u8> = (0..200_000).map(|n| (n % 251) as u8).collect();
    fixture.put("profile/Documents/data.bin", &data);
    fixture.put("profile/empty.txt", b"");
    fs::create_dir(fixture.root.join("profile/empty-dir")).unwrap();

    let before = sha256(
        &fixture.root.join("profile/Documents/data.bin"),
        &AtomicBool::new(false),
    )
    .unwrap();

    let mut progress = Vec::new();

    let result = recover(
        &fixture.root,
        &["profile".into()],
        &fixture.destination,
        &AtomicBool::new(false),
        |copied_bytes, total| progress.push((copied_bytes, total)),
    );

    assert_eq!(
        result.operation.state,
        State::Completed,
        "{:?}",
        result.operation.errors
    );

    assert_eq!(result.files.len(), 2);
    assert_eq!(result.copied_bytes, 200_000);
    assert!(result.files.iter().all(|file| file.verified));
    assert_eq!(
        fs::read(fixture.destination.join("profile/Documents/data.bin")).unwrap(),
        data
    );

    assert!(fixture.destination.join("profile/empty-dir").is_dir());
    assert_eq!(
        before,
        sha256(
            &fixture.root.join("profile/Documents/data.bin"),
            &AtomicBool::new(false)
        )
        .unwrap()
    );

    assert!(progress.windows(2).all(|v| v[0].0 < v[1].0));
    assert_eq!(progress.last(), Some(&(200_000, 200_000)));
}

#[test]
fn same_volume_rejected_before_writes() {
    let fixture = Fixture::new();
    fixture.put("a", b"original");
    let sibling = fixture.root.with_extension("same-volume");
    fs::create_dir(&sibling).unwrap();

    let result = recover(
        &fixture.root,
        &["a".into()],
        &sibling,
        &AtomicBool::new(false),
        |_, _| {},
    );

    assert_eq!(result.operation.state, State::Failed);
    assert!(!sibling.join("a").exists());
    assert_eq!(fs::read(fixture.root.join("a")).unwrap(), b"original");

    fs::remove_dir(sibling).unwrap();
}

#[test]
fn nested_destination_rejected() {
    let fixture = Fixture::new();
    fixture.put("folder/a", b"original");

    let result = recover(
        &fixture.root,
        &["folder".into()],
        &fixture.root.join("folder"),
        &AtomicBool::new(false),
        |_, _| {},
    );

    assert_eq!(result.operation.state, State::Failed);
    assert!(!fixture.root.join("folder/folder").exists());
}

#[test]
fn existing_destination_never_overwritten() {
    let fixture = Fixture::new();
    fixture.put("a", b"source");
    fs::write(fixture.destination.join("a"), b"existing").unwrap();

    let result = fixture.copy(&["a"]);

    assert_eq!(result.operation.errors[0].kind, ErrorKind::Conflict);
    assert_eq!(
        fs::read(fixture.destination.join("a")).unwrap(),
        b"existing"
    );
    assert_eq!(fs::read(fixture.root.join("a")).unwrap(), b"source");
}

#[test]
fn insufficient_space_uses_actual_free_space() {
    let fixture = Fixture::new();
    let file = File::create(fixture.root.join("sparse.bin")).unwrap();
    let available = space(&fixture.destination).unwrap().available;
    file.set_len(available + 1).unwrap();

    let result = fixture.copy(&["sparse.bin"]);

    assert_eq!(result.operation.state, State::Failed);
    assert!(result.operation.errors[0].context.contains("Insufficient"));
    assert!(!fixture.destination.join("sparse.bin").exists());
    assert_eq!(file.metadata().unwrap().len(), available + 1);
}

#[cfg(unix)]
#[test]
fn unreadable_source_is_not_silently_skipped() {
    let fixture = Fixture::new();
    fixture.put("denied", b"private fixture");
    let path = fixture.root.join("denied");
    fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

    let result = fixture.copy(&["denied"]);
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();

    assert_eq!(result.operation.state, State::Failed);
    assert_eq!(result.operation.errors[0].kind, ErrorKind::Permission);
    assert_eq!(fs::read(path).unwrap(), b"private fixture");
    assert!(!fixture.destination.join("denied").exists());
}

#[test]
fn cancellation_reports_exact_partial_bytes() {
    let fixture = Fixture::new();
    let data = vec![42u8; 200_000];
    fixture.put("a", &data);
    let cancel = AtomicBool::new(false);

    let result = recover(
        &fixture.root,
        &["a".into()],
        &fixture.destination,
        &cancel,
        |copied_bytes, _| {
            if copied_bytes >= 65536 {
                cancel.store(true, Ordering::Relaxed)
            }
        },
    );

    assert_eq!(result.operation.state, State::Cancelled);
    assert_eq!(result.copied_bytes, 65536);
    assert_eq!(
        fs::metadata(fixture.destination.join("a")).unwrap().len(),
        65536
    );
    assert!(!result.files[0].verified);
    assert_eq!(fs::read(fixture.root.join("a")).unwrap(), data);
}

#[test]
fn cancellation_before_preflight_writes_nothing() {
    let fixture = Fixture::new();
    fixture.put("a", b"original");

    let result = recover(
        &fixture.root,
        &["a".into()],
        &fixture.destination,
        &AtomicBool::new(true),
        |_, _| {},
    );

    assert_eq!(result.operation.state, State::Cancelled);
    assert_eq!(fs::read_dir(&fixture.destination).unwrap().count(), 0);
}

#[test]
fn partway_conflict_reports_partial_result() {
    let fixture = Fixture::new();
    fixture.put("a", b"first");
    fixture.put("b", b"second");

    let result = recover(
        &fixture.root,
        &["a".into(), "b".into()],
        &fixture.destination,
        &AtomicBool::new(false),
        |_, _| {
            fs::write(fixture.destination.join("b"), b"injected conflict").unwrap();
        },
    );

    assert_eq!(result.operation.state, State::Failed);
    assert_eq!(result.copied_bytes, 5);
    assert!(!result.files[0].verified);
    assert_eq!(fs::read(fixture.root.join("a")).unwrap(), b"first");
    assert_eq!(fs::read(fixture.root.join("b")).unwrap(), b"second");
}

#[test]
fn destination_write_error_after_progress_leaves_unverified_partial() {
    let fixture = Fixture::new();
    let source_data = vec![42; 200_000];
    fixture.put("a", &source_data);
    fixture.put("b", b"second");
    let existing_file = fixture.destination.join("unrelated");
    fs::write(&existing_file, b"existing destination content").unwrap();
    let source_path = fixture.root.join("a");
    let source_hash = Sha256::digest(fs::read(&source_path).unwrap());
    let first_chunk = 65_536;
    let mut actual_bytes_written = 0;
    let mut injected_errors = 0;
    let mut progress = Vec::new();

    let result = recover_with_writer(
        &fixture.root,
        &["a".into(), "b".into()],
        &fixture.destination,
        &AtomicBool::new(false),
        |copied_bytes, planned_bytes| progress.push((copied_bytes, planned_bytes)),
        |output, buffer| {
            if actual_bytes_written == first_chunk {
                injected_errors += 1;
                return Err(io::Error::other("injected destination write failure"));
            }

            let bytes_written = output.write(buffer)?;
            actual_bytes_written += bytes_written;

            Ok(bytes_written)
        },
    );

    assert_eq!(injected_errors, 1);
    assert_eq!(actual_bytes_written, first_chunk);
    assert_eq!(progress.last(), Some(&(first_chunk as u64, 200_006)));
    assert_eq!(result.operation.state, State::Failed);
    assert_eq!(
        result.operation.transitions,
        [
            State::Pending,
            State::Validating,
            State::Running,
            State::Failed
        ]
    );
    assert_eq!(result.operation.errors.len(), 1);
    let error = &result.operation.errors[0];
    assert_eq!(error.kind, ErrorKind::Io);
    assert_eq!(
        error.context,
        "Write destination bytes: injected destination write failure"
    );
    assert_eq!(result.planned_bytes, 200_006);
    assert_eq!(result.copied_bytes, first_chunk as u64);
    assert_eq!(result.files.len(), 1);
    let partial = &result.files[0];
    assert_eq!(partial.relative, Path::new("a"));
    assert_eq!(partial.bytes, first_chunk as u64);
    assert!(!partial.verified);
    assert!(partial.sha256.is_none());

    assert_eq!(
        fs::read(fixture.destination.join("a")).unwrap(),
        source_data[..first_chunk]
    );
    assert!(!fixture.destination.join("b").exists());
    assert_eq!(Sha256::digest(fs::read(source_path).unwrap()), source_hash);
    assert_eq!(fs::read(fixture.root.join("b")).unwrap(), b"second");
    assert_eq!(
        fs::read(existing_file).unwrap(),
        b"existing destination content"
    );
}

#[test]
fn destination_corruption_is_caught_by_readback() {
    let fixture = Fixture::new();
    fixture.put("a", b"source bytes");

    let result = recover(
        &fixture.root,
        &["a".into()],
        &fixture.destination,
        &AtomicBool::new(false),
        |_, _| fs::write(fixture.destination.join("a"), b"corrupted!!!").unwrap(),
    );

    assert_eq!(result.operation.state, State::Failed);
    assert_eq!(result.operation.errors[0].kind, ErrorKind::Integrity);
    assert!(!result.files[0].verified);
    assert_eq!(fs::read(fixture.root.join("a")).unwrap(), b"source bytes");
}

#[cfg(unix)]
#[test]
fn symlink_source_and_destination_ancestors_are_rejected() {
    let fixture = Fixture::new();
    fixture.put("a", b"source");
    symlink(fixture.root.join("a"), fixture.root.join("link")).unwrap();

    assert_eq!(fixture.copy(&["link"]).operation.state, State::Blocked);

    fixture.put("sub/b", b"second");
    symlink(&fixture.root, fixture.destination.join("sub")).unwrap();

    let result = fixture.copy(&["sub/b"]);

    assert_ne!(result.operation.state, State::Completed);
    assert_eq!(fs::read(fixture.root.join("a")).unwrap(), b"source");
}

#[test]
fn distinct_unicode_case_collisions_are_rejected() {
    let fixture = Fixture::new();
    fixture.put("tree/k.txt", b"ASCII name");
    let kelvin_name = "tree/\u{212a}.txt";
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fixture.root.join(kelvin_name))
        .unwrap()
        .write_all(b"Kelvin sign name")
        .unwrap();
    assert_eq!(fs::read_dir(fixture.root.join("tree")).unwrap().count(), 2);
    assert_eq!(
        fs::read(fixture.root.join("tree/k.txt")).unwrap(),
        b"ASCII name"
    );
    assert_eq!(
        fs::read(fixture.root.join(kelvin_name)).unwrap(),
        b"Kelvin sign name"
    );

    for selections in [vec!["tree"], vec!["tree/k.txt", kelvin_name]] {
        let result = fixture.copy(&selections);

        assert_eq!(result.operation.state, State::Failed);
        assert_eq!(result.operation.errors[0].kind, ErrorKind::Conflict);
        assert_eq!(result.copied_bytes, 0);
        assert_eq!(fs::read_dir(&fixture.destination).unwrap().count(), 0);
    }
}

#[cfg(unix)]
#[test]
fn distinct_ascii_case_collisions_are_rejected() {
    let fixture = Fixture::new();
    fixture.put("tree/a", b"lowercase");
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(fixture.root.join("tree/A"))
        .unwrap()
        .write_all(b"uppercase")
        .unwrap();
    assert_eq!(fs::read_dir(fixture.root.join("tree")).unwrap().count(), 2);

    let result = fixture.copy(&["tree"]);

    assert_eq!(result.operation.state, State::Failed);
    assert_eq!(result.operation.errors[0].kind, ErrorKind::Conflict);
    assert_eq!(result.copied_bytes, 0);
    assert_eq!(fs::read_dir(&fixture.destination).unwrap().count(), 0);
}

#[cfg(windows)]
#[test]
fn case_alias_selections_are_rejected_on_default_ntfs() {
    let fixture = Fixture::new();
    fixture.put("tree/a", b"one entry");
    let lowercase = fixture.root.join("tree/a");
    let uppercase = fixture.root.join("tree/A");
    let alias_creation = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&uppercase)
        .unwrap_err();
    assert_eq!(alias_creation.kind(), std::io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read_dir(fixture.root.join("tree")).unwrap().count(), 1);
    assert_eq!(
        fs::canonicalize(lowercase).unwrap(),
        fs::canonicalize(uppercase).unwrap()
    );

    let result = fixture.copy(&["tree/a", "tree/A"]);

    assert_eq!(result.operation.state, State::Failed);
    assert_eq!(result.operation.errors[0].kind, ErrorKind::Conflict);
    assert_eq!(result.copied_bytes, 0);
    assert_eq!(fs::read_dir(&fixture.destination).unwrap().count(), 0);
}

#[test]
fn overlapping_selections_are_rejected_without_a_collision() {
    let fixture = Fixture::new();
    fixture.put("tree/a", b"child");

    let result = fixture.copy(&["tree", "tree/a"]);

    assert_eq!(result.operation.state, State::Failed);
    assert_eq!(result.operation.errors[0].kind, ErrorKind::Validation);
    assert_eq!(result.operation.errors[0].context, "Overlapping selections");
    assert_eq!(result.copied_bytes, 0);
    assert_eq!(fs::read_dir(&fixture.destination).unwrap().count(), 0);
}

#[test]
fn transaction_restarts_partial_and_rechecks_published_files() {
    let fixture = Fixture::new();
    fixture.put("a-empty", b"");
    fixture.put("b-ação", &vec![13; 300_000]);
    fs::create_dir(fixture.root.join("empty-directory")).unwrap();
    let cancel = AtomicBool::new(false);
    let selections = ["a-empty".into(), "b-ação".into(), "empty-directory".into()];
    let directory =
        transaction::prepare(&fixture.root, &selections, &fixture.destination, &cancel).unwrap();

    let stopped = transaction::resume(&fixture.root, &directory, &cancel, |progress| {
        if progress.acknowledged_bytes >= 65_536 {
            cancel.store(true, Ordering::Relaxed);
        }
    })
    .unwrap();

    assert_eq!(stopped.status.state, State::Cancelled);
    assert_eq!(stopped.progress.verified_files, 1);
    assert!(directory.join("payload/a-empty").is_file());
    assert!(!directory.join("payload/b-ação").exists());
    assert_eq!(
        fs::metadata(directory.join("journal/1.part"))
            .unwrap()
            .len(),
        65_536
    );

    cancel.store(false, Ordering::Relaxed);
    let resumed = transaction::resume(&fixture.root, &directory, &cancel, |_| {}).unwrap();

    assert!(resumed.status.resumed);
    assert_eq!(resumed.status.state, State::Completed);
    assert_eq!(resumed.progress.skipped_files, 1);
    assert_eq!(resumed.progress.verified_bytes, 300_000);
    assert!(directory.join("payload/empty-directory").is_dir());
    assert_eq!(
        fs::read(directory.join("payload/b-ação")).unwrap(),
        vec![13; 300_000]
    );
    assert!(!directory.join("journal/1.part").exists());

    transaction::verify(&fixture.root, &directory, &cancel).unwrap();
    fs::write(directory.join("payload/b-ação"), vec![14; 300_000]).unwrap();

    assert!(transaction::verify(&fixture.root, &directory, &cancel).is_err());
    let failed = transaction::resume(&fixture.root, &directory, &cancel, |_| {}).unwrap();
    assert_eq!(failed.status.state, State::Failed);
    assert_eq!(
        fs::read(directory.join("payload/b-ação")).unwrap(),
        vec![14; 300_000]
    );
}

#[test]
fn transaction_refuses_changed_source_destination_conflict_and_corrupt_manifest() {
    let fixture = Fixture::new();
    fixture.put("data", b"original");
    let cancel = AtomicBool::new(false);
    let directory = transaction::prepare(
        &fixture.root,
        &["data".into()],
        &fixture.destination,
        &cancel,
    )
    .unwrap();

    fixture.put("data", b"modified");
    let changed = transaction::resume(&fixture.root, &directory, &cancel, |_| {}).unwrap();
    assert_eq!(changed.status.state, State::Failed);
    assert_eq!(
        changed.files[0].state,
        transaction::FileState::SourceChanged
    );
    assert!(!directory.join("payload/data").exists());

    fixture.put("data", b"original");
    fs::write(directory.join("payload/data"), b"original").unwrap();
    let conflict = transaction::resume(&fixture.root, &directory, &cancel, |_| {}).unwrap();
    assert_eq!(conflict.status.state, State::Failed);
    assert_eq!(
        conflict.files[0].state,
        transaction::FileState::DestinationConflict
    );
    assert_eq!(
        fs::read(directory.join("payload/data")).unwrap(),
        b"original"
    );

    fs::write(directory.join("manifest.json"), b"{\"sha256\":").unwrap();
    assert!(transaction::resume(&fixture.root, &directory, &cancel, |_| {}).is_err());
    assert!(transaction::checkpoints(&fixture.destination).unwrap()[0]
        .error
        .is_some());
}

#[test]
fn transaction_rejects_duplicate_selection_and_moved_operation() {
    let fixture = Fixture::new();
    fixture.put("data", b"original");
    let cancel = AtomicBool::new(false);

    assert!(transaction::prepare(
        &fixture.root,
        &["data".into(), "data".into()],
        &fixture.destination,
        &cancel
    )
    .is_err());
    assert!(transaction::prepare(&fixture.root, &["data".into()], &fixture.root, &cancel).is_err());

    let directory = transaction::prepare(
        &fixture.root,
        &["data".into()],
        &fixture.destination,
        &cancel,
    )
    .unwrap();
    let other_parent = fixture.destination.join("different");
    fs::create_dir(&other_parent).unwrap();
    let moved = other_parent.join(directory.file_name().unwrap());
    fs::rename(directory, &moved).unwrap();

    assert!(transaction::resume(&fixture.root, &moved, &cancel, |_| {}).is_err());
    assert!(!moved.join("payload/data").exists());
}
