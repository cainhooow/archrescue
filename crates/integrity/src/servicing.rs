//! Bounded metadata samples and contextual evidence from the current CBS log.

use std::{
    collections::BTreeSet,
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use super::*;

const SAMPLE_LIMIT: usize = 24;
const ENTRY_LIMIT: usize = 100_000;
const XML_LIMIT: u64 = 2 * 1024 * 1024;
const LOG_LIMIT: u64 = 512 * 1024;

pub(super) fn inspect(report: &mut IntegrityReport, cancel: &AtomicBool) -> Result<()> {
    for (relative, extension) in [
        ("WinSxS/Manifests", "manifest"),
        ("servicing/Packages", "mum"),
    ] {
        check_cancel(cancel)?;
        let path = report.windows_directory.join(relative);
        let result = match metadata(&path) {
            Ok(Some(metadata)) if metadata.is_dir() => {
                match sample_directory(report, &path, extension, cancel) {
                    Ok(count) => evidence(
                        Observation::VerifiedStructure,
                        format!(
                            "Diretório acessível. {count} arquivos consultados por amostragem."
                        ),
                    ),
                    Err(error) if error.kind == ErrorKind::Cancelled => return Err(error),
                    Err(error) => unavailable(error),
                }
            }
            Ok(Some(_)) => evidence(
                Observation::ConfirmedCorruption,
                "A estrutura de manutenção exige um diretório neste caminho.",
            ),
            Ok(None) => evidence(
                Observation::ConfirmedMissing,
                "Diretório de metadados de manutenção ausente no perfil observado.",
            ),
            Err(error) => unavailable(error),
        };
        record(report, Category::ComponentStore, relative, path, result);
    }

    check_cancel(cancel)?;
    let path = report.windows_directory.join("Logs/CBS/CBS.log");
    match metadata(&path) {
        Ok(None) => report
            .not_executed
            .push("Histórico CBS: CBS.log ausente.".into()),
        Ok(Some(metadata)) if metadata.is_file() => {
            if let Err(error) = inspect_log(report, &path, cancel) {
                if error.kind == ErrorKind::Cancelled {
                    return Err(error);
                }
                record(
                    report,
                    Category::ServicingHistory,
                    "CBS.log",
                    path,
                    unavailable(error),
                );
            }
        }
        Ok(Some(_)) => record(
            report,
            Category::ServicingHistory,
            "CBS.log",
            path,
            evidence(
                Observation::Inconclusive,
                "O caminho do log não é um arquivo regular.",
            ),
        ),
        Err(error) => record(
            report,
            Category::ServicingHistory,
            "CBS.log",
            path,
            unavailable(error),
        ),
    }

    Ok(())
}

pub(super) fn sample_directory(
    report: &mut IntegrityReport,
    path: &Path,
    extension: &str,
    cancel: &AtomicBool,
) -> Result<usize> {
    let path = safe_existing(path)?;
    let mut sample = BTreeSet::new();
    let mut catalogs = BTreeSet::new();
    let entries = fs::read_dir(&path).map_err(unreadable)?;

    for (index, entry) in entries.enumerate() {
        check_cancel(cancel)?;
        if index == ENTRY_LIMIT {
            return Err(Error::new(
                ErrorKind::Incomplete,
                "Limite de entradas de manutenção atingido",
            ));
        }
        let entry = entry.map_err(unreadable)?;
        let entry_path = entry.path();
        let suffix = entry_path.extension().and_then(|value| value.to_str());
        if suffix.is_some_and(|suffix| suffix.eq_ignore_ascii_case(extension)) {
            retain_sample(&mut sample, entry_path);
        } else if extension == "mum"
            && suffix.is_some_and(|suffix| suffix.eq_ignore_ascii_case("cat"))
        {
            retain_sample(&mut catalogs, entry_path);
        }
    }

    if sample.is_empty() {
        record(
            report,
            Category::ComponentStore,
            extension,
            path.clone(),
            evidence(
                Observation::SuspectedCorruption,
                "Nenhum manifesto no diretório de metadados esperado.",
            ),
        );
    }

    for file in &sample {
        check_cancel(cancel)?;
        let component = file.file_name().unwrap_or_default().to_string_lossy();
        record(
            report,
            Category::ComponentStore,
            &component,
            file.clone(),
            inspect_xml(file, extension),
        );
    }

    for catalog in catalogs {
        check_cancel(cancel)?;
        let Some(identity) = catalog
            .file_stem()
            .and_then(|value| value.to_str())
            .filter(|name| package_name(name))
        else {
            continue;
        };
        let manifest = catalog.with_extension("mum");
        if metadata(&manifest).is_ok_and(|metadata| metadata.is_none()) {
            let component = format!("{identity}.mum");
            record(
                report,
                Category::ComponentStore,
                &component,
                manifest,
                evidence(
                    Observation::SuspectedCorruption,
                    format!(
                        "Catálogo observado sem manifesto de pacote correspondente: {}",
                        catalog.display()
                    ),
                ),
            );
        }
    }

    Ok(sample.len())
}

fn retain_sample(sample: &mut BTreeSet<PathBuf>, path: PathBuf) {
    sample.insert(path);
    if sample.len() > SAMPLE_LIMIT {
        sample.pop_last();
    }
}

fn inspect_xml(path: &Path, extension: &str) -> ResultEvidence {
    let bytes = match read_regular(path, XML_LIMIT) {
        Ok(bytes) => bytes,
        Err(error) => return unavailable(error),
    };
    if extension == "manifest" && bytes.starts_with(b"DCM\x01") {
        return evidence(
            Observation::Unsupported,
            "Manifesto WinSxS comprimido; XML não inspecionado.",
        );
    }

    match decode(&bytes) {
        Some(text) => xml_evidence(&text),
        None => evidence(
            Observation::Unsupported,
            "Codificação do manifesto fora do perfil de leitura.",
        ),
    }
}

fn xml_evidence(xml: &str) -> ResultEvidence {
    use Observation::*;

    let document = match roxmltree::Document::parse_with_options(
        xml,
        roxmltree::ParsingOptions {
            nodes_limit: 50_000,
            ..Default::default()
        },
    ) {
        Ok(document) => document,
        Err(error @ (roxmltree::Error::DtdDetected | roxmltree::Error::NodesLimitReached)) => {
            return evidence(
                Unsupported,
                format!("XML fora dos limites do leitor: {error}"),
            );
        }
        Err(error) => return evidence(SuspectedCorruption, format!("XML ilegível: {error}")),
    };
    let root = document.root_element();
    let identity = root
        .children()
        .find(|node| node.has_tag_name("assemblyIdentity"));
    if root.tag_name().name() != "assembly"
        || !identity.is_some_and(|node| {
            node.attribute("name").is_some_and(|name| !name.is_empty())
                && node
                    .attribute("version")
                    .is_some_and(|version| !version.is_empty())
        })
    {
        return evidence(
            SuspectedCorruption,
            "XML sem estrutura assembly/assemblyIdentity esperada.",
        );
    }

    evidence(
        VerifiedStructure,
        "XML e identidade de assembly legíveis. Catálogo não autenticado.",
    )
}

fn read_regular(path: &Path, limit: u64) -> Result<Vec<u8>> {
    let path = safe_existing(path)?;
    let metadata = fs::metadata(&path).map_err(unreadable)?;
    if !metadata.is_file() || metadata.len() > limit {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Tipo ou tamanho de metadados fora do limite de leitura",
        ));
    }

    let mut bytes = Vec::new();
    File::open(&path)
        .map_err(unreadable)?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;
    if bytes.len() as u64 > limit {
        return Err(Error::new(
            ErrorKind::Incomplete,
            "Metadados excederam o limite durante a leitura",
        ));
    }

    Ok(bytes)
}

fn decode(bytes: &[u8]) -> Option<String> {
    if let Some(utf16) = bytes
        .strip_prefix(&[0xff, 0xfe])
        .or_else(|| bytes.strip_prefix(&[0xfe, 0xff]))
    {
        if utf16.len() % 2 != 0 {
            return None;
        }
        let little_endian = bytes[0] == 0xff;
        let words = utf16
            .chunks_exact(2)
            .map(|pair| {
                let pair = [pair[0], pair[1]];
                if little_endian {
                    u16::from_le_bytes(pair)
                } else {
                    u16::from_be_bytes(pair)
                }
            })
            .collect::<Vec<_>>();

        return String::from_utf16(&words).ok();
    }

    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    std::str::from_utf8(bytes).ok().map(str::to_owned)
}

fn inspect_log(report: &mut IntegrityReport, path: &Path, cancel: &AtomicBool) -> Result<()> {
    let path = safe_existing(path)?;
    let mut file = File::open(&path).map_err(unreadable)?;
    let offset = file
        .metadata()
        .map_err(unreadable)?
        .len()
        .saturating_sub(LOG_LIMIT);
    file.seek(SeekFrom::Start(offset)).map_err(unreadable)?;
    let mut bytes = Vec::new();
    file.take(LOG_LIMIT)
        .read_to_end(&mut bytes)
        .map_err(unreadable)?;

    // Rotated and UTF-16 logs are outside this profile. Discard a partial first line.
    let first_line = if offset == 0 {
        0
    } else {
        bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(bytes.len(), |index| index + 1)
    };
    let text = decode(&bytes[first_line..])
        .filter(|text| !text.contains('\0'))
        .ok_or_else(|| {
            Error::new(
                ErrorKind::Unsupported,
                "Codificação do histórico CBS fora do perfil",
            )
        })?;
    let mut count = 0;
    let mut seen = BTreeSet::new();

    for line in text.lines().rev() {
        check_cancel(cancel)?;
        if let Some(entry) = cbs_entry(line) {
            if !seen.insert((entry.code, entry.package, entry.operation)) {
                continue;
            }
            count += 1;
            record(
                report,
                Category::ServicingHistory,
                &format!("{} · {} · {}", entry.code, entry.operation, entry.package),
                path.clone(),
                evidence(
                    Observation::HistoricalFailure,
                    format!(
                        "{} · {} · {} · {} · {}. Registro histórico.\n{}",
                        entry.timestamp,
                        entry.operation,
                        entry.package,
                        entry.code,
                        entry.meaning,
                        line
                    ),
                ),
            );
            if count == SAMPLE_LIMIT {
                break;
            }
        }
    }

    record(
        report,
        Category::ServicingHistory,
        "CBS.log",
        path,
        evidence(
            Observation::VerifiedStructure,
            format!(
                "{} bytes da cauda do log analisados. {count} falhas com contexto reconhecido.",
                bytes.len() - first_line,
            ),
        ),
    );

    Ok(())
}

#[derive(Debug)]
pub(super) struct CbsEntry<'a> {
    pub timestamp: &'a str,
    pub code: &'a str,
    pub package: &'a str,
    pub operation: &'static str,
    pub meaning: &'static str,
}

pub(super) fn cbs_entry(line: &str) -> Option<CbsEntry<'_>> {
    if line.len() > 4096 {
        return None;
    }
    let (timestamp, body) = line.split_once(',')?;
    let shape = b"0000-00-00 00:00:00";
    if timestamp.len() != shape.len()
        || !timestamp.bytes().zip(shape).all(|(actual, expected)| {
            if *expected == b'0' {
                actual.is_ascii_digit()
            } else {
                actual == *expected
            }
        })
    {
        return None;
    }
    let body = body.trim_start();
    let (level, body) = body.split_once(char::is_whitespace)?;
    if !matches!(level, "Info" | "Error") {
        return None;
    }
    let body = body.trim_start().strip_prefix("CBS")?.trim_start();
    let (operation, details) = [
        ("Failed to resolve package ", "Resolução de pacote"),
        ("Failed to internally open package ", "Abertura de pacote"),
        ("Failed to process package ", "Processamento de pacote"),
    ]
    .into_iter()
    .find_map(|(prefix, action)| body.strip_prefix(prefix).map(|detail| (action, detail)))?;
    let (package, hresult) = details.split_once(" [HRESULT = ")?;
    if !package_name(package) {
        return None;
    }
    let (code, _) = hresult.split_once(" - ")?;
    let meaning = match code.to_ascii_uppercase().as_str() {
        "0X800F081F" => "origem de reparo não encontrada",
        "0X800F0831" | "0X80073712" => "repositório inconsistente",
        "0X800736CC" => "hash de arquivo de componente divergente",
        "0X80070002" => "arquivo não encontrado",
        "0X8007000D" => "dados inválidos",
        _ => return None,
    };
    if !hresult.ends_with(']') {
        return None;
    }

    Some(CbsEntry {
        timestamp,
        code,
        package,
        operation,
        meaning,
    })
}

fn package_name(name: &str) -> bool {
    let parts: Vec<_> = name.split('~').collect();
    let [package, token, architecture, language, version] = parts.as_slice() else {
        return false;
    };
    let name_chars = |value: &str| {
        value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    };
    !package.is_empty()
        && name_chars(package)
        && token.len() == 16
        && token.bytes().all(|byte| byte.is_ascii_hexdigit())
        && matches!(
            *architecture,
            "amd64" | "x86" | "arm64" | "wow64" | "msil" | "neutral"
        )
        && name_chars(language)
        && version.split('.').count() == 4
        && version.split('.').all(|part| part.parse::<u32>().is_ok())
}
