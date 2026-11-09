//! Synthetic filesystem fixtures exercise real parsers. They are not licensed Windows images.
use archrescue_core::Operation;
use std::{fs, path::PathBuf, sync::atomic::AtomicBool};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../.agent-tmp/contracts")
            .join(Operation::new().id);
        fs::create_dir_all(&root).unwrap();
        Self(fs::canonicalize(root).unwrap())
    }
    fn put(&self, path: &str, bytes: &[u8]) {
        let p = self.0.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn valid(f: &Fixture) {
    f.put("Windows/System32/config/SYSTEM", b"regf fixture only");
    f.put("Windows/System32/config/SOFTWARE", b"regf fixture only");
    let mut pe = vec![0u8; 134];
    pe[..2].copy_from_slice(b"MZ");
    pe[60..64].copy_from_slice(&128u32.to_le_bytes());
    pe[128..132].copy_from_slice(b"PE\0\0");
    pe[132..134].copy_from_slice(&0x8664u16.to_le_bytes());
    f.put("Windows/System32/ntoskrnl.exe", &pe);
}
#[test]
fn no_windows_is_an_empty_result() {
    let f = Fixture::new();
    let r = archrescue_windows_detection::discover(std::slice::from_ref(&f.0));
    assert!(r.installations.is_empty());
    assert!(r.errors.is_empty());
}
#[test]
fn directory_named_windows_is_not_an_installation() {
    let f = Fixture::new();
    fs::create_dir(f.0.join("Windows")).unwrap();
    let r = archrescue_windows_detection::discover(std::slice::from_ref(&f.0));
    assert!(r.installations.is_empty());
    assert!(!r.errors.is_empty());
}
#[test]
fn corroborated_candidate_has_architecture_not_invented_version() {
    let f = Fixture::new();
    valid(&f);
    let r = archrescue_windows_detection::discover(std::slice::from_ref(&f.0));
    assert_eq!(r.installations.len(), 1);
    assert_eq!(r.installations[0].architecture.as_deref(), Some("x64"));
    assert_eq!(r.installations[0].version, None);
    assert!(!r.installations[0].limitations.is_empty());
}
#[test]
fn malformed_pe_header_is_rejected() {
    let f = Fixture::new();
    valid(&f);
    f.put("Windows/System32/ntoskrnl.exe", b"MZbroken");
    assert!(
        archrescue_windows_detection::discover(std::slice::from_ref(&f.0))
            .installations
            .is_empty()
    );
}
#[test]
fn malformed_registry_marker_is_rejected() {
    let f = Fixture::new();
    valid(&f);
    f.put("Windows/System32/config/SYSTEM", b"wrong signature");
    assert!(
        archrescue_windows_detection::discover(std::slice::from_ref(&f.0))
            .installations
            .is_empty()
    );
}
#[test]
fn missing_and_malformed_bcd_are_not_healthy() {
    let f = Fixture::new();
    let empty = archrescue_diagnostics::inspect(std::slice::from_ref(&f.0));
    assert_eq!(empty.observations.len(), 4);
    assert!(empty.observations.iter().all(|o| o.status == "missing"));
    f.put("EFI/Microsoft/Boot/BCD", b"broken");
    let r = archrescue_diagnostics::inspect(std::slice::from_ref(&f.0));
    assert!(!r.errors.is_empty());
    assert!(r.observations.iter().any(|o| o.status == "unavailable"));
}
#[cfg(unix)]
#[test]
fn unreadable_boot_configuration_is_reported() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    f.put("Boot/BCD", b"regf fixture");
    fs::set_permissions(f.0.join("Boot/BCD"), fs::Permissions::from_mode(0o000)).unwrap();
    let r = archrescue_diagnostics::inspect(std::slice::from_ref(&f.0));
    fs::set_permissions(f.0.join("Boot/BCD"), fs::Permissions::from_mode(0o600)).unwrap();
    assert!(r
        .errors
        .iter()
        .any(|e| e.kind == archrescue_core::ErrorKind::Permission));
}
#[test]
fn explorer_lists_actual_names_sizes_and_directories() {
    let f = Fixture::new();
    f.put("folder/data.txt", b"abc");
    f.put("empty", b"");
    let entries = archrescue_backup::browse(&f.0).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].directory);
    assert_eq!(entries[0].name, PathBuf::from("folder"));
    let nested = archrescue_backup::browse(&f.0.join("folder")).unwrap();
    assert_eq!(nested[0].bytes, 3);
    assert_eq!(nested[0].name, PathBuf::from("data.txt"));
}
#[test]
fn report_export_roundtrips_and_preserves_source() {
    let f = Fixture::new();
    valid(&f);
    let before = archrescue_backup::sha256(
        &f.0.join("Windows/System32/ntoskrnl.exe"),
        &AtomicBool::new(false),
    )
    .unwrap();
    let output = Fixture::new();
    let report = archrescue_evidence::Report::collect(std::slice::from_ref(&f.0));
    let path = archrescue_evidence::export(&report, &output.0).unwrap();
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
            &f.0.join("Windows/System32/ntoskrnl.exe"),
            &AtomicBool::new(false)
        )
        .unwrap()
    );
    let second = archrescue_evidence::export(&report, &output.0).unwrap();
    assert_ne!(path, second);
}
