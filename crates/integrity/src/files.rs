//! Independent installation markers and structural PE validation.

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use archrescue_windows_detection::{pe_architecture, pe_file_version};

use super::*;

const KERNEL: &str = "ntoskrnl.exe";
const NTDLL: &str = "ntdll.dll";
const SESSION: &str = "smss.exe";
const CORE: [&str; 5] = [KERNEL, NTDLL, SESSION, "services.exe", "lsass.exe"];

pub(super) fn profile(windows: &Path) -> Result<Option<WindowsIdentity>> {
    #[cfg(windows)]
    {
        let running = std::env::var_os("SystemRoot").ok_or_else(|| {
            Error::new(
                ErrorKind::Environment,
                "SystemRoot do ambiente ativo indisponível",
            )
        })?;
        let running = safe_existing(Path::new(&running))?;
        if running == windows || identity(&running)?.volume == identity(windows)?.volume {
            return Err(Error::new(
                ErrorKind::Environment,
                "Volume do Windows em execução excluído da análise offline",
            ));
        }
    }

    let system = safe_existing(&windows.join("System32"))?;
    for hive in ["SYSTEM", "SOFTWARE"] {
        let path = safe_existing(&system.join("config").join(hive))?;
        let mut signature = [0; 4];
        File::open(path)
            .and_then(|mut file| file.read_exact(&mut signature))
            .map_err(unreadable)?;
        if &signature != b"regf" {
            return Ok(None);
        }
    }

    // Two independent x64 files are enough to qualify a damaged kernel for inspection.
    let markers = [KERNEL, NTDLL, SESSION].map(|name| system.join(name));
    let valid = markers
        .iter()
        .filter(|path| {
            pe_architecture(path).is_ok_and(|machine| machine == "x64")
                && pe_structure(path, &[]).is_ok()
        })
        .collect::<Vec<_>>();

    if valid.len() < 2 {
        return Ok(None);
    }

    Ok(valid.into_iter().find_map(|path| {
        let version = pe_file_version(path)?;
        if !modern_version(&version) {
            return None;
        }
        Some(WindowsIdentity {
            architecture: "x64".into(),
            file_version: version,
            version_source: path.clone(),
        })
    }))
}

pub(super) fn modern_version(version: &str) -> bool {
    let parts = version.split('.').collect::<Vec<_>>();
    let [major, minor, build, revision] = parts.as_slice() else {
        return false;
    };
    // WinPE can report a 6.2 compatibility prefix on a modern kernel resource.
    // The older 6.2 builds are outside the accepted Windows build range.
    matches!((*major, *minor), ("10", "0") | ("6", "2"))
        && build
            .parse::<u32>()
            .is_ok_and(|build| (10240..30000).contains(&build))
        && revision.parse::<u32>().is_ok()
}

pub(super) fn inspect(report: &mut IntegrityReport, cancel: &AtomicBool) -> Result<()> {
    let system = report.windows_directory.join("System32");
    for name in CORE {
        check_cancel(cancel)?;
        let path = system.join(name);
        let result = executable(&path, &[]);
        record(report, Category::System, name, path, result);
    }

    // Windows boot applications can use the native boot subsystem, including on UEFI.
    let loaders: [(&str, &[u16]); 2] = [("winload.efi", &[10, 16]), ("winload.exe", &[16])];
    let results = loaders.map(|(name, subsystem)| executable(&system.join(name), subsystem));
    let both_missing = results
        .iter()
        .all(|(state, _, _)| *state == Observation::ConfirmedMissing);

    for ((name, _), mut result) in loaders.into_iter().zip(results) {
        if !both_missing && result.0 == Observation::ConfirmedMissing {
            result = evidence(
                Observation::Unsupported,
                "Arquivo ausente. O modo de inicialização ativo precisa ser confirmado antes de exigir este carregador.",
            );
        }
        record(report, Category::Boot, name, system.join(name), result);
    }

    Ok(())
}

fn executable(path: &Path, subsystems: &[u16]) -> ResultEvidence {
    match metadata(path) {
        Ok(None) => {
            return evidence(
                Observation::ConfirmedMissing,
                "O caminho exigido está ausente no volume acessível.",
            )
        }
        Ok(Some(metadata)) if !metadata.is_file() => {
            return evidence(
                Observation::ConfirmedCorruption,
                "O caminho exigido não é um arquivo regular.",
            );
        }
        Err(error) => return unavailable(error),
        _ => {}
    }

    match pe_structure(path, subsystems) {
        Ok(()) => evidence(Observation::VerifiedStructure, "Cabeçalhos DOS, COFF, PE32+ e limites de seções verificados. Catálogo e conteúdo não autenticados."),
        Err(error) if error.kind == ErrorKind::Integrity => (
            Observation::ConfirmedCorruption, error.context.clone(), Some(error)
        ),
        Err(error) => unavailable(error),
    }
}

fn malformed(message: &str) -> Error {
    Error::new(ErrorKind::Integrity, message)
}

fn read_header(file: &mut File, header: &mut [u8]) -> Result<()> {
    file.read_exact(header).map_err(|error| {
        if error.kind() == io::ErrorKind::UnexpectedEof {
            malformed("Cabeçalho PE truncado.")
        } else {
            unreadable(error)
        }
    })
}

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap())
}

fn dword(bytes: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap())
}

pub(super) fn pe_structure(path: &Path, subsystems: &[u16]) -> Result<()> {
    let path = safe_existing(path)?;
    let mut file = File::open(path).map_err(unreadable)?;
    let length = file.metadata().map_err(unreadable)?.len();

    let mut dos = [0; 64];
    read_header(&mut file, &mut dos)?;
    let offset = u64::from(dword(&dos, 60));
    if &dos[..2] != b"MZ" || offset < 64 || offset + 24 > length {
        return Err(malformed(
            "Assinatura DOS ou deslocamento do cabeçalho PE inválido.",
        ));
    }

    file.seek(SeekFrom::Start(offset)).map_err(unreadable)?;
    let mut coff = [0; 24];
    read_header(&mut file, &mut coff)?;
    let sections = u64::from(word(&coff, 6));
    let optional_size = u64::from(word(&coff, 20));
    if &coff[..4] != b"PE\0\0"
        || word(&coff, 4) != 0x8664
        || !(1..=96).contains(&sections)
        || optional_size < 112
        || offset + 24 + optional_size + sections * 40 > length
    {
        return Err(malformed(
            "Cabeçalho COFF x64 incompatível ou tabela de seções truncada.",
        ));
    }

    let mut optional = [0; 112];
    read_header(&mut file, &mut optional)?;
    if word(&optional, 0) != 0x20b
        || (!subsystems.is_empty() && !subsystems.contains(&word(&optional, 68)))
    {
        return Err(malformed("Formato PE32+ ou subsistema esperado inválido."));
    }

    file.seek(SeekFrom::Start(offset + 24 + optional_size))
        .map_err(unreadable)?;
    let mut section = [0; 40];
    for _ in 0..sections {
        read_header(&mut file, &mut section)?;
        let size = u64::from(dword(&section, 16));
        let start = u64::from(dword(&section, 20));
        if size != 0 && (start == 0 || start + size > length) {
            return Err(malformed("Dados de seção fora dos limites do arquivo PE."));
        }
    }

    Ok(())
}
