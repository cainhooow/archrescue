//! Controlled fixtures only. Cross-filesystem tests require an explicit disposable destination.
use super::*;
#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};
struct Fixture {
    root: PathBuf,
    dest: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let id = Operation::new().id;
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.agent-tmp/tests");
        fs::create_dir_all(&base).unwrap();
        let root = base.join(&id);
        fs::create_dir(&root).unwrap();
        let external=std::env::var_os("ARCHRESCUE_TEST_DESTINATION").expect("Set ARCHRESCUE_TEST_DESTINATION to a disposable directory on a separate filesystem; see scripts/test.sh");
        let dest = PathBuf::from(external).join(&id);
        fs::create_dir(&dest).unwrap();
        assert_ne!(
            identity(&root).unwrap().volume,
            identity(&dest).unwrap().volume,
            "Tests require two real filesystems"
        );
        Self {
            root: fs::canonicalize(root).unwrap(),
            dest: fs::canonicalize(dest).unwrap(),
        }
    }
    fn put(&self, name: &str, data: &[u8]) {
        let p = self.root.join(name);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, data).unwrap();
    }
    fn copy(&self, selections: &[&str]) -> BackupResult {
        recover(
            &self.root,
            &selections.iter().map(PathBuf::from).collect::<Vec<_>>(),
            &self.dest,
            &AtomicBool::new(false),
            |_, _| {},
        )
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
        let _ = fs::remove_dir_all(&self.dest);
    }
}
#[test]
fn portable_paths_reject_traversal_and_devices() {
    for p in [
        "../secret",
        "/absolute",
        "NUL",
        "dir/CON.txt",
        "name:stream",
        "trailing.",
    ] {
        assert!(portable_relative(Path::new(p)).is_err(), "{p}");
    }
}
#[test]
fn verified_copy_preserves_hierarchy_empty_dirs_and_source() {
    let f = Fixture::new();
    let data: Vec<u8> = (0..200_000).map(|n| (n % 251) as u8).collect();
    f.put("profile/Documents/data.bin", &data);
    f.put("profile/empty.txt", b"");
    fs::create_dir(f.root.join("profile/empty-dir")).unwrap();
    let before = sha256(
        &f.root.join("profile/Documents/data.bin"),
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut progress = Vec::new();
    let r = recover(
        &f.root,
        &["profile".into()],
        &f.dest,
        &AtomicBool::new(false),
        |n, total| progress.push((n, total)),
    );
    assert_eq!(
        r.operation.state,
        State::Completed,
        "{:?}",
        r.operation.errors
    );
    assert_eq!(r.files.len(), 2);
    assert_eq!(r.copied_bytes, 200_000);
    assert!(r.files.iter().all(|f| f.verified));
    assert_eq!(
        fs::read(f.dest.join("profile/Documents/data.bin")).unwrap(),
        data
    );
    assert!(f.dest.join("profile/empty-dir").is_dir());
    assert_eq!(
        before,
        sha256(
            &f.root.join("profile/Documents/data.bin"),
            &AtomicBool::new(false)
        )
        .unwrap()
    );
    assert!(progress.windows(2).all(|v| v[0].0 < v[1].0));
    assert_eq!(progress.last(), Some(&(200_000, 200_000)));
}
#[test]
fn same_volume_rejected_before_writes() {
    let f = Fixture::new();
    f.put("a", b"original");
    let sibling = f.root.with_extension("same-volume");
    fs::create_dir(&sibling).unwrap();
    let r = recover(
        &f.root,
        &["a".into()],
        &sibling,
        &AtomicBool::new(false),
        |_, _| {},
    );
    assert_eq!(r.operation.state, State::Failed);
    assert!(!sibling.join("a").exists());
    assert_eq!(fs::read(f.root.join("a")).unwrap(), b"original");
    fs::remove_dir(sibling).unwrap();
}
#[test]
fn nested_destination_rejected() {
    let f = Fixture::new();
    f.put("folder/a", b"original");
    let r = recover(
        &f.root,
        &["folder".into()],
        &f.root.join("folder"),
        &AtomicBool::new(false),
        |_, _| {},
    );
    assert_eq!(r.operation.state, State::Failed);
    assert!(!f.root.join("folder/folder").exists());
}
#[test]
fn existing_destination_never_overwritten() {
    let f = Fixture::new();
    f.put("a", b"source");
    fs::write(f.dest.join("a"), b"existing").unwrap();
    let r = f.copy(&["a"]);
    assert_eq!(r.operation.errors[0].kind, ErrorKind::Conflict);
    assert_eq!(fs::read(f.dest.join("a")).unwrap(), b"existing");
    assert_eq!(fs::read(f.root.join("a")).unwrap(), b"source");
}
#[test]
fn insufficient_space_uses_actual_free_space() {
    let f = Fixture::new();
    let file = File::create(f.root.join("sparse.bin")).unwrap();
    let available = space(&f.dest).unwrap().available;
    file.set_len(available + 1).unwrap();
    let r = f.copy(&["sparse.bin"]);
    assert_eq!(r.operation.state, State::Failed);
    assert!(r.operation.errors[0].context.contains("Insufficient"));
    assert!(!f.dest.join("sparse.bin").exists());
    assert_eq!(file.metadata().unwrap().len(), available + 1);
}
#[cfg(unix)]
#[test]
fn unreadable_source_is_not_silently_skipped() {
    let f = Fixture::new();
    f.put("denied", b"private fixture");
    let p = f.root.join("denied");
    fs::set_permissions(&p, fs::Permissions::from_mode(0o000)).unwrap();
    let r = f.copy(&["denied"]);
    fs::set_permissions(&p, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(r.operation.state, State::Failed);
    assert_eq!(r.operation.errors[0].kind, ErrorKind::Permission);
    assert_eq!(fs::read(p).unwrap(), b"private fixture");
    assert!(!f.dest.join("denied").exists());
}
#[test]
fn cancellation_reports_exact_partial_bytes() {
    let f = Fixture::new();
    let data = vec![42u8; 200_000];
    f.put("a", &data);
    let cancel = AtomicBool::new(false);
    let r = recover(&f.root, &["a".into()], &f.dest, &cancel, |n, _| {
        if n >= 65536 {
            cancel.store(true, Ordering::Relaxed)
        }
    });
    assert_eq!(r.operation.state, State::Cancelled);
    assert_eq!(r.copied_bytes, 65536);
    assert_eq!(fs::metadata(f.dest.join("a")).unwrap().len(), 65536);
    assert!(!r.files[0].verified);
    assert_eq!(fs::read(f.root.join("a")).unwrap(), data);
}
#[test]
fn cancellation_before_preflight_writes_nothing() {
    let f = Fixture::new();
    f.put("a", b"original");
    let r = recover(
        &f.root,
        &["a".into()],
        &f.dest,
        &AtomicBool::new(true),
        |_, _| {},
    );
    assert_eq!(r.operation.state, State::Cancelled);
    assert_eq!(fs::read_dir(&f.dest).unwrap().count(), 0);
}
#[test]
fn partway_conflict_reports_partial_result() {
    let f = Fixture::new();
    f.put("a", b"first");
    f.put("b", b"second");
    let r = recover(
        &f.root,
        &["a".into(), "b".into()],
        &f.dest,
        &AtomicBool::new(false),
        |_, _| {
            fs::write(f.dest.join("b"), b"injected conflict").unwrap();
        },
    );
    assert_eq!(r.operation.state, State::Failed);
    assert_eq!(r.copied_bytes, 5);
    assert!(!r.files[0].verified);
    assert_eq!(fs::read(f.root.join("a")).unwrap(), b"first");
    assert_eq!(fs::read(f.root.join("b")).unwrap(), b"second");
}
#[test]
fn unavailable_destination_never_completes() {
    let f = Fixture::new();
    f.put("a", b"first");
    f.put("b", b"second");
    let offline = f.dest.with_extension("offline");
    let r = recover(
        &f.root,
        &["a".into(), "b".into()],
        &f.dest,
        &AtomicBool::new(false),
        |_, _| fs::rename(&f.dest, &offline).unwrap(),
    );
    fs::rename(offline, &f.dest).unwrap();
    assert_eq!(r.operation.state, State::Failed);
    assert_eq!(r.copied_bytes, 5);
    assert!(!f.dest.join("b").exists());
    assert_eq!(fs::read(f.root.join("b")).unwrap(), b"second");
}
#[test]
fn destination_corruption_is_caught_by_readback() {
    let f = Fixture::new();
    f.put("a", b"source bytes");
    let r = recover(
        &f.root,
        &["a".into()],
        &f.dest,
        &AtomicBool::new(false),
        |_, _| fs::write(f.dest.join("a"), b"corrupted!!!").unwrap(),
    );
    assert_eq!(r.operation.state, State::Failed);
    assert_eq!(r.operation.errors[0].kind, ErrorKind::Integrity);
    assert!(!r.files[0].verified);
    assert_eq!(fs::read(f.root.join("a")).unwrap(), b"source bytes");
}
#[cfg(unix)]
#[test]
fn symlink_source_and_destination_ancestors_are_rejected() {
    let f = Fixture::new();
    f.put("a", b"source");
    symlink(f.root.join("a"), f.root.join("link")).unwrap();
    assert_eq!(f.copy(&["link"]).operation.state, State::Blocked);
    f.put("sub/b", b"second");
    symlink(&f.root, f.dest.join("sub")).unwrap();
    let r = f.copy(&["sub/b"]);
    assert_ne!(r.operation.state, State::Completed);
    assert_eq!(fs::read(f.root.join("a")).unwrap(), b"source");
}
#[test]
fn case_collisions_and_overlapping_selections_are_rejected() {
    let f = Fixture::new();
    f.put("tree/a", b"a");
    f.put("tree/A", b"b");
    assert_ne!(f.copy(&["tree"]).operation.state, State::Completed);
    assert_ne!(
        f.copy(&["tree", "tree/a"]).operation.state,
        State::Completed
    );
    assert_eq!(fs::read_dir(&f.dest).unwrap().count(), 0);
}
