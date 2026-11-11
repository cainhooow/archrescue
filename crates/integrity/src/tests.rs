use archrescue_core::Operation;

use super::*;

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path =
            std::env::temp_dir().join(format!("archrescue-integrity-{}", Operation::new().id));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn rejects_truncation_and_wrong_machine_without_reading_payload() {
    let fixture = Fixture::new();
    let image = fixture.0.join("winload.efi");
    let mut bytes = vec![0; 256];
    bytes[..2].copy_from_slice(b"MZ");
    bytes[60] = 64;
    bytes[64..70].copy_from_slice(b"PE\0\0\x64\x86");
    bytes[70] = 1;
    bytes[84] = 112;
    bytes[88..90].copy_from_slice(&0x20b_u16.to_le_bytes());
    bytes[156] = 10;
    fs::write(&image, &bytes).unwrap();

    assert!(files::pe_structure(&image, Some(10)).is_ok());
    assert_eq!(
        files::pe_structure(&image, Some(1)).unwrap_err().kind,
        ErrorKind::Integrity
    );

    fs::write(&image, &bytes[..190]).unwrap();
    assert_eq!(
        files::pe_structure(&image, None).unwrap_err().kind,
        ErrorKind::Integrity
    );

    bytes[68..70].copy_from_slice(&0xaa64_u16.to_le_bytes());
    fs::write(&image, &bytes).unwrap();
    assert_eq!(
        files::pe_structure(&image, None).unwrap_err().kind,
        ErrorKind::Integrity
    );
}

#[test]
fn cancellation_never_returns_clean_status() {
    let fixture = Fixture::new();
    let report = scan(
        &fixture.0,
        ScanMode::PassiveOffline,
        &AtomicBool::new(true),
        |_| {},
    );

    assert_eq!(report.status, Status::Cancelled);
    assert!(report.checks.is_empty());
    assert_eq!(report.errors[0].kind, ErrorKind::Cancelled);
}

#[test]
fn missing_or_unqualified_windows_never_reports_clean() {
    let fixture = Fixture::new();
    let cancel = AtomicBool::new(false);
    let missing = scan(&fixture.0, ScanMode::PassiveOffline, &cancel, |_| {});
    assert_eq!(missing.status, Status::Blocked);

    let system = fixture.0.join("Windows/System32/config");
    fs::create_dir_all(&system).unwrap();
    fs::write(system.join("SYSTEM"), b"regf").unwrap();
    fs::write(system.join("SOFTWARE"), b"regf").unwrap();

    let unsupported = scan(&fixture.0, ScanMode::PassiveOffline, &cancel, |_| {});
    #[cfg(not(windows))]
    assert_eq!(unsupported.status, Status::Unsupported);
    #[cfg(windows)]
    assert!(matches!(
        unsupported.status,
        Status::Blocked | Status::Unsupported
    ));
    assert!(unsupported.checks.is_empty());
}

#[test]
fn historical_errors_require_a_recognized_operation_and_package() {
    let package = "Package_for_RollupFix~31bf3856ad364e35~amd64~~10.0.19041.1";
    let line = format!("2020-10-19 21:39:22, Info CBS Failed to resolve package {package} [HRESULT = 0x800f0831 - CBS_E_STORE_CORRUPTION]");
    let entry = servicing::cbs_entry(&line).unwrap();
    assert_eq!(entry.package, package);
    assert_eq!(entry.code, "0x800f0831");
    assert_eq!(entry.timestamp, "2020-10-19 21:39:22");
    assert!(!Observation::HistoricalFailure.actionable());

    assert!(servicing::cbs_entry("2020-10-19 21:39:22, Info CBS help: 0x800f0831").is_none());
    assert!(servicing::cbs_entry(&line.replace(package, "..\\untrusted")).is_none());
    assert!(servicing::cbs_entry(&line.replace("2020-10-19", "yesterday ")).is_none());
}

#[test]
fn metadata_sample_distinguishes_malformed_xml_and_compression() {
    let fixture = Fixture::new();
    let valid = fixture.0.join("valid.manifest");
    let broken = fixture.0.join("broken.manifest");
    let compressed = fixture.0.join("compressed.manifest");

    fs::write(&valid, br#"<assembly xmlns="urn:schemas-microsoft-com:asm.v3"><assemblyIdentity name="sample" version="10.0.1.0"/></assembly>"#).unwrap();
    fs::write(&broken, b"<assembly><assemblyIdentity").unwrap();
    fs::write(&compressed, b"DCM\x01opaque").unwrap();

    let cancel = AtomicBool::new(false);
    let mut report = scan(&fixture.0, ScanMode::PassiveOffline, &cancel, |_| {});
    servicing::sample_directory(&mut report, &fixture.0, "manifest", &cancel).unwrap();

    let state = |path: &Path| {
        report
            .checks
            .iter()
            .find(|check| check.path.ends_with(path.file_name().unwrap()))
            .unwrap()
            .observation
    };
    assert_eq!(state(&valid), Observation::VerifiedStructure);
    assert_eq!(state(&broken), Observation::SuspectedCorruption);
    assert_eq!(state(&compressed), Observation::Unsupported);
}

#[cfg(unix)]
#[test]
fn reparse_like_entries_are_inconclusive_instead_of_missing() {
    let fixture = Fixture::new();
    let redirected = fixture.0.join("external");
    std::os::unix::fs::symlink("absent", &redirected).unwrap();

    let error = metadata(&redirected.join("ntoskrnl.exe")).unwrap_err();
    assert_eq!(error.kind, ErrorKind::Unsupported);
}
