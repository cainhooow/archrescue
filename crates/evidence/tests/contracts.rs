//! Synthetic filesystem fixtures exercise real parsers. They are not licensed Windows images.

use std::{fs, path::PathBuf, sync::atomic::AtomicBool};

use archrescue_core::Operation;

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.agent-tmp/contracts")
            .join(Operation::new().id);
        fs::create_dir_all(&root).unwrap();

        Self {
            root: fs::canonicalize(root).unwrap(),
        }
    }

    fn put(&self, path: &str, bytes: &[u8]) {
        let path = self.root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn write_windows_markers(fixture: &Fixture) {
    fixture.put("Windows/System32/config/SYSTEM", b"regf fixture only");
    fixture.put("Windows/System32/config/SOFTWARE", b"regf fixture only");

    let mut pe = vec![0u8; 134];
    pe[..2].copy_from_slice(b"MZ");
    pe[60..64].copy_from_slice(&128u32.to_le_bytes());
    pe[128..132].copy_from_slice(b"PE\0\0");
    pe[132..134].copy_from_slice(&0x8664u16.to_le_bytes());

    fixture.put("Windows/System32/ntoskrnl.exe", &pe);
}

#[test]
fn no_windows_is_an_empty_result() {
    let fixture = Fixture::new();

    let result = archrescue_windows_detection::discover(std::slice::from_ref(&fixture.root));

    assert!(result.installations.is_empty());
    assert!(result.errors.is_empty());
}

#[test]
fn directory_named_windows_is_not_an_installation() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.join("Windows")).unwrap();

    let result = archrescue_windows_detection::discover(std::slice::from_ref(&fixture.root));

    assert!(result.installations.is_empty());
    assert!(!result.errors.is_empty());
}

#[test]
fn corroborated_candidate_has_architecture_not_invented_version() {
    let fixture = Fixture::new();
    write_windows_markers(&fixture);

    let result = archrescue_windows_detection::discover(std::slice::from_ref(&fixture.root));

    assert_eq!(result.installations.len(), 1);
    assert_eq!(result.installations[0].architecture.as_deref(), Some("x64"));
    assert_eq!(result.installations[0].version, None);
    assert!(!result.installations[0].limitations.is_empty());
}

#[test]
fn malformed_pe_header_is_rejected() {
    let fixture = Fixture::new();
    write_windows_markers(&fixture);
    fixture.put("Windows/System32/ntoskrnl.exe", b"MZbroken");

    assert!(
        archrescue_windows_detection::discover(std::slice::from_ref(&fixture.root))
            .installations
            .is_empty()
    );
}

#[test]
fn malformed_registry_marker_is_rejected() {
    let fixture = Fixture::new();
    write_windows_markers(&fixture);
    fixture.put("Windows/System32/config/SYSTEM", b"wrong signature");

    assert!(
        archrescue_windows_detection::discover(std::slice::from_ref(&fixture.root))
            .installations
            .is_empty()
    );
}

#[test]
fn missing_and_malformed_bcd_are_not_healthy() {
    let fixture = Fixture::new();

    let empty = archrescue_diagnostics::inspect(std::slice::from_ref(&fixture.root));

    assert_eq!(empty.observations.len(), 4);
    assert!(empty
        .observations
        .iter()
        .all(|observation| observation.status == "missing"));

    fixture.put("EFI/Microsoft/Boot/BCD", b"broken");

    let result = archrescue_diagnostics::inspect(std::slice::from_ref(&fixture.root));

    assert!(!result.errors.is_empty());
    assert!(result
        .observations
        .iter()
        .any(|observation| observation.status == "unavailable"));
}

#[cfg(unix)]
#[test]
fn unreadable_boot_configuration_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fixture.put("Boot/BCD", b"regf fixture");
    fs::set_permissions(
        fixture.root.join("Boot/BCD"),
        fs::Permissions::from_mode(0o000),
    )
    .unwrap();

    let result = archrescue_diagnostics::inspect(std::slice::from_ref(&fixture.root));
    fs::set_permissions(
        fixture.root.join("Boot/BCD"),
        fs::Permissions::from_mode(0o600),
    )
    .unwrap();

    assert!(result
        .errors
        .iter()
        .any(|error| error.kind == archrescue_core::ErrorKind::Permission));
}

#[test]
fn explorer_lists_actual_names_sizes_and_directories() {
    let fixture = Fixture::new();
    fixture.put("folder/data.txt", b"abc");
    fixture.put("empty", b"");

    let entries = archrescue_backup::browse(&fixture.root).unwrap();

    assert_eq!(entries.len(), 2);
    assert!(entries[0].directory);
    assert_eq!(entries[0].name, PathBuf::from("folder"));

    let nested = archrescue_backup::browse(&fixture.root.join("folder")).unwrap();

    assert_eq!(nested[0].bytes, 3);
    assert_eq!(nested[0].name, PathBuf::from("data.txt"));
}

#[test]
fn report_export_roundtrips_and_preserves_source() {
    let fixture = Fixture::new();
    write_windows_markers(&fixture);
    let before = archrescue_backup::sha256(
        &fixture.root.join("Windows/System32/ntoskrnl.exe"),
        &AtomicBool::new(false),
    )
    .unwrap();

    let output = Fixture::new();
    let report = archrescue_evidence::Report::collect(std::slice::from_ref(&fixture.root));

    let path = archrescue_evidence::export(&report, &output.root).unwrap();
    let json = fs::read(path.join("report.json")).unwrap();
    let decoded: archrescue_evidence::Report = serde_json::from_slice(&json).unwrap();

    assert_eq!(decoded.schema_version, 1);
    assert_eq!(decoded.installations.installations.len(), 1);
    assert!(fs::read_to_string(path.join("report.txt"))
        .unwrap()
        .contains("Technical report"));

    assert_eq!(
        before,
        archrescue_backup::sha256(
            &fixture.root.join("Windows/System32/ntoskrnl.exe"),
            &AtomicBool::new(false)
        )
        .unwrap()
    );

    let second = archrescue_evidence::export(&report, &output.root).unwrap();

    assert_ne!(path, second);
}

#[test]
fn diagnosis_classifies_candidates_loader_mismatch_windows_old_and_invalid_bcd() {
    let first = Fixture::new();
    let second = Fixture::new();
    let incomplete = Fixture::new();
    let absent = Fixture::new();
    write_windows_markers(&first);
    write_windows_markers(&second);
    fs::create_dir(first.root.join("Windows.old")).unwrap();
    fs::create_dir(incomplete.root.join("Windows")).unwrap();

    let mut loader = fs::read(first.root.join("Windows/System32/ntoskrnl.exe")).unwrap();
    loader[132..134].copy_from_slice(&0xaa64u16.to_le_bytes());
    first.put("Windows/System32/winload.efi", &loader);
    first.put("Windows/System32/Recovery/ReAgent.xml", b"<WindowsRE />");
    first.put("EFI/Microsoft/Boot/BCD", b"invalid");

    let report = archrescue_evidence::Report::collect(&[
        first.root.clone(),
        second.root.clone(),
        incomplete.root.clone(),
        absent.root.clone(),
        absent.root.join("unavailable-volume"),
    ]);
    assert_eq!(report.installations.installations.len(), 2);
    for expected in [
        "WINDOWS_LOADER_ARCH_MISMATCH",
        "WINDOWS_OLD_PRESENT",
        "WINDOWS_RE_FILES_PRESENT",
        "WINDOWS_MARKERS_INCOMPLETE",
        "WINDOWS_ABSENT",
        "WINDOWS_VOLUME_UNAVAILABLE",
        "EFI_IMAGE_PATH_MISSING",
        "BCD_INVALID_SIGNATURE",
        "MULTIPLE_WINDOWS",
    ] {
        assert!(
            report
                .diagnosis
                .findings
                .iter()
                .any(|finding| finding.id == expected),
            "{expected}"
        );
    }
}
