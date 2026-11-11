use std::{fs, sync::atomic::Ordering};

use archrescue_backup::transaction;
use archrescue_core::Operation;

use super::*;

struct Fixture {
    root: PathBuf,
    backup_destination: PathBuf,
    request: Request,
    profile: Profile,
    source: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let id = Operation::new().id;
        let root = std::env::temp_dir().join(&id);
        let destination = std::env::var_os("ARCHRESCUE_TEST_DESTINATION")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/dev/shm"));
        let backup_destination = destination.join(&id);
        fs::create_dir(&backup_destination).unwrap();

        let source = root.join("source");
        let documents = source.join("Old/Documents");
        fs::create_dir_all(documents.join("empty folder")).unwrap();
        fs::write(documents.join("a-empty.txt"), []).unwrap();
        fs::write(documents.join("b-ação.bin"), vec![7; 2_500_000]).unwrap();
        let profile_root = root.join("new-profile");
        let target = profile_root.join("My Documents");
        fs::create_dir_all(&target).unwrap();

        let cancel = AtomicBool::new(false);
        let backup = transaction::prepare(
            &source,
            &["Old/Documents".into()],
            &backup_destination,
            &cancel,
        )
        .unwrap();
        let report = transaction::resume(&source, &backup, &cancel, |_| {}).unwrap();
        assert!(report.status.error.is_none(), "{:?}", report.status.error);

        let profile = Profile {
            account_sid: "test-account".into(),
            root: profile_root,
            folders: BTreeMap::from([(Folder::Documents, target)]),
        };
        let request = Request {
            backup,
            target_sid: profile.account_sid.clone(),
            conflicts: ConflictPolicy::Abort,
            mappings: vec![Mapping {
                source: "Old/Documents".into(),
                folder: Folder::Documents,
            }],
        };

        Self {
            root,
            backup_destination,
            request,
            profile,
            source,
        }
    }

    fn target(&self) -> &Path {
        &self.profile.folders[&Folder::Documents]
    }

    fn preview(&self) -> Result<RestorePlan> {
        let (plan, _, _) = prepare(&self.request, &self.profile, &AtomicBool::new(false))?;
        Ok(plan)
    }

    fn run(
        &self,
        plan: &RestorePlan,
        cancel: &AtomicBool,
        progress: impl FnMut(usize, usize, &FileReport),
    ) -> Result<RestoreReport> {
        execution::execute_for(
            &self.request,
            &self.profile,
            &format!("AUTORIZAR-RESTAURACAO:{}", plan.fingerprint),
            cancel,
            progress,
        )
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
        fs::remove_dir_all(&self.backup_destination).unwrap();
    }
}

#[test]
fn verified_restore_preserves_unicode_empty_and_multichunk_files() {
    let fixture = Fixture::new();
    let plan = fixture.preview().unwrap();
    let cancel = AtomicBool::new(false);

    let report = fixture.run(&plan, &cancel, |_, _, _| {}).unwrap();

    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.files.len(), 2);
    for entry in report.files {
        let original = fixture.source.join(&entry.source_relative);
        let restored = fixture.profile.root.join(entry.target_relative.unwrap());
        assert_eq!(fs::read(original).unwrap(), fs::read(restored).unwrap());
        assert_eq!(
            entry.actual_sha256.as_deref(),
            Some(entry.expected_sha256.as_str())
        );
        assert_eq!(entry.outcome, FileOutcome::Verified);
        assert!(entry.published);
    }
    assert!(fixture.target().join("empty folder").is_dir());
}

#[test]
fn abort_skip_and_rename_preserve_existing_content() {
    let mut fixture = Fixture::new();
    let conflict = fixture.target().join("a-empty.txt");
    fs::write(&conflict, b"existing").unwrap();
    let cancel = AtomicBool::new(false);

    let plan = fixture.preview().unwrap();
    assert!(plan.blocked_by_conflicts);
    let report = fixture.run(&plan, &cancel, |_, _, _| {}).unwrap();
    assert_eq!(report.files[0].outcome, FileOutcome::Conflicted);
    assert!(!fixture.target().join("b-ação.bin").exists());

    fixture.request.conflicts = ConflictPolicy::Skip;
    let plan = fixture.preview().unwrap();
    let report = fixture.run(&plan, &cancel, |_, _, _| {}).unwrap();
    assert_eq!(report.files[0].outcome, FileOutcome::Skipped);
    assert!(!report.files[0].published);

    fixture.request.conflicts = ConflictPolicy::Rename;
    let plan = fixture.preview().unwrap();
    let report = fixture.run(&plan, &cancel, |_, _, _| {}).unwrap();
    assert!(report.error.is_none(), "{:?}", report.error);
    assert_eq!(report.files[0].action, Action::Rename);
    assert_eq!(fs::read(&conflict).unwrap(), b"existing");
    assert_eq!(
        fs::read(fixture.target().join("a-empty (ArchRescue 1).txt")).unwrap(),
        b""
    );
}

#[test]
fn tampered_and_missing_payload_fail_preflight() {
    let fixture = Fixture::new();
    let path = fixture
        .request
        .backup
        .join("payload/Old/Documents/b-ação.bin");
    fs::write(&path, vec![8; 2_500_000]).unwrap();

    assert!(fixture.preview().is_err());
    fs::remove_file(path).unwrap();
    assert!(fixture.preview().is_err());
    assert!(!fixture.target().join("a-empty.txt").exists());
}

#[test]
fn corrupt_and_unsupported_manifest_fail_preflight() {
    let fixture = Fixture::new();
    let path = fixture.request.backup.join("manifest.json");
    let original = fs::read_to_string(&path).unwrap();
    fs::write(
        &path,
        original
            .replace("\"schema_version\":1", "\"schema_version\":9")
            .replace("\"schema_version\": 1", "\"schema_version\": 9"),
    )
    .unwrap();
    assert_ne!(fs::read_to_string(&path).unwrap(), original);
    assert!(fixture.preview().is_err());

    fs::write(path, "{broken").unwrap();
    assert!(fixture.preview().is_err());
}

#[test]
fn rejects_wrong_sid_traversal_external_mapping_and_unavailable_path() {
    let mut fixture = Fixture::new();
    fixture.request.target_sid = "other-account".into();
    assert_eq!(fixture.preview().unwrap_err().kind, ErrorKind::Permission);

    fixture.request.target_sid = fixture.profile.account_sid.clone();
    fixture.request.mappings[0].source = "../Documents".into();
    assert!(fixture.preview().is_err());

    fixture.request.mappings[0].source = "Old/Documents".into();
    fixture
        .profile
        .folders
        .insert(Folder::Documents, fixture.source.clone());
    assert!(fixture.preview().is_err());

    fixture
        .profile
        .folders
        .insert(Folder::Documents, fixture.profile.root.join("unavailable"));
    assert!(fixture.preview().is_err());
    assert!(require_capacity(&fixture.profile.root, u64::MAX).is_err());
}

#[cfg(unix)]
#[test]
fn rejects_a_symlink_in_a_mapped_destination() {
    let fixture = Fixture::new();
    std::os::unix::fs::symlink(&fixture.source, fixture.target().join("empty folder")).unwrap();

    assert_eq!(fixture.preview().unwrap_err().kind, ErrorKind::Unsupported);
}

#[test]
fn authorization_is_invalidated_when_known_folder_identity_changes() {
    let fixture = Fixture::new();
    let plan = fixture.preview().unwrap();
    fs::rename(fixture.target(), fixture.profile.root.join("previous")).unwrap();
    fs::create_dir(fixture.target()).unwrap();

    let error = fixture
        .run(&plan, &AtomicBool::new(false), |_, _, _| {})
        .unwrap_err();

    assert_eq!(error.kind, ErrorKind::Permission);
    assert!(!fixture.target().join("a-empty.txt").exists());
}

#[test]
fn copy_after_preflight_rejects_new_hash_mismatch_and_cleans_staging() {
    let fixture = Fixture::new();
    let plan = fixture.preview().unwrap();
    let cancel = AtomicBool::new(false);
    let payload = fixture
        .request
        .backup
        .join("payload/Old/Documents/b-ação.bin");

    let report = fixture
        .run(&plan, &cancel, |done, _, _| {
            if done == 1 {
                fs::write(&payload, vec![9; 2_500_000]).unwrap();
            }
        })
        .unwrap();

    assert_eq!(report.files[0].outcome, FileOutcome::Verified);
    assert_eq!(report.files[1].outcome, FileOutcome::Failed);
    assert_eq!(report.error.unwrap().kind, ErrorKind::Integrity);
    assert!(!fixture.target().join("b-ação.bin").exists());
    assert!(!fs::read_dir(fixture.target()).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .ends_with(".part")));
}

#[test]
fn cancellation_stops_before_the_next_file() {
    let fixture = Fixture::new();
    let plan = fixture.preview().unwrap();
    let cancel = AtomicBool::new(false);

    let report = fixture
        .run(&plan, &cancel, |done, _, _| {
            if done == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        })
        .unwrap();

    assert_eq!(report.error.unwrap().kind, ErrorKind::Cancelled);
    assert_eq!(report.files[0].outcome, FileOutcome::Verified);
    assert_eq!(report.files[1].outcome, FileOutcome::NotAttempted);
}
