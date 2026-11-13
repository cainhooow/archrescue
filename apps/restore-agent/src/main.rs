use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use archrescue_core::{Error, ErrorKind, Result, BUILD_ID, VERSION};
use archrescue_restore::{ConflictPolicy, Folder, Mapping, Profile, RecoveryBinding, Request};

const HELP: &str = "Agente de Restauração ArchRescue
  --version
  perfil
  previa BACKUP SID POLÍTICA PASTA_ORIGEM PASTA_CONHECIDA [...]
  restaurar BACKUP SID POLÍTICA AUTORIZAR-RESTAURACAO:SHA256 PASTA_ORIGEM PASTA_CONHECIDA [...]
  previa-recuperacao REGISTRO BACKUP SID POLÍTICA PASTA_ORIGEM PASTA_CONHECIDA [...]
  restaurar-recuperacao REGISTRO BACKUP SID POLÍTICA AUTORIZAR-RESTAURACAO:SHA256 PASTA_ORIGEM PASTA_CONHECIDA [...]

Execute com a conta de destino. Use caminho absoluto para BACKUP.
Política: abortar, ignorar, renomear.
Pastas conhecidas: Desktop, Documents, Downloads, Pictures.
Repita o par de pastas para cada mapeamento. A origem é relativa ao manifesto.
Revise a prévia e use a impressão digital do plano para autorizar a restauração.";

fn main() {
    if let Err(error) = run() {
        eprintln!("Falha: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let invalid = || {
        Error::new(
            ErrorKind::Validation,
            "Argumentos inválidos. Consulte --help",
        )
    };
    let command = args
        .first()
        .and_then(|value| value.to_str())
        .unwrap_or("--help");
    let cancel = AtomicBool::new(false);
    match command {
        "--help" if args.len() <= 1 => println!("{HELP}"),
        "--version" if args.len() == 1 => {
            println!("ArchRescue Restore Agent {VERSION} build={BUILD_ID}")
        }
        "perfil" if args.len() == 1 => print_json(&Profile::current()?)?,
        "previa" | "restaurar" | "previa-recuperacao" | "restaurar-recuperacao" => {
            let executing = command.starts_with("restaurar");
            let with_recovery = command.ends_with("-recuperacao");
            let offset = usize::from(with_recovery);
            let start = if executing { 5 } else { 4 };
            if args.len() < offset + start + 2 || (args.len() - offset - start) % 2 != 0 {
                return Err(invalid());
            }
            let recovery = with_recovery
                .then(|| read_binding(Path::new(&args[1])))
                .transpose()?;
            let args = &args[offset..];
            let conflicts = match args[3].to_str() {
                Some("abortar") => ConflictPolicy::Abort,
                Some("ignorar") => ConflictPolicy::Skip,
                Some("renomear") => ConflictPolicy::Rename,
                _ => return Err(invalid()),
            };

            let mut mappings = Vec::new();
            for pair in args[start..].as_chunks::<2>().0 {
                mappings.push(Mapping {
                    source: PathBuf::from(&pair[0]),
                    folder: Folder::parse(pair[1].to_str().ok_or_else(invalid)?)?,
                });
            }
            let request = Request {
                backup: PathBuf::from(&args[1]),
                target_sid: args[2].to_str().ok_or_else(invalid)?.into(),
                mappings,
                conflicts,
            };

            if executing {
                let report = archrescue_restore::execute_recovery(
                    &request,
                    recovery.as_ref(),
                    args[4].to_str().ok_or_else(invalid)?,
                    &cancel,
                    |done, total, file| {
                        eprintln!(
                            "{done}/{total} {:?} {}",
                            file.outcome,
                            file.source_relative.display()
                        )
                    },
                )?;
                print_json(&report)?;
                if let Some(error) = report.error {
                    return Err(error);
                }
            } else {
                print_json(&archrescue_restore::preview_recovery(
                    &request,
                    recovery.as_ref(),
                    &cancel,
                )?)?;
            }
        }
        _ => return Err(invalid()),
    }

    Ok(())
}

fn print_json(value: &impl serde::Serialize) -> Result<()> {
    let stdout = std::io::stdout().lock();
    serde_json::to_writer_pretty(stdout, value)
        .map_err(|error| Error::new(ErrorKind::Io, error.to_string()))?;
    println!();

    Ok(())
}

#[derive(serde::Deserialize)]
struct DeploymentRecord {
    schema_version: u32,
    operation_id: String,
    plan_fingerprint: String,
    execution_state: String,
    restore: RecoveryBinding,
    errors: Vec<Error>,
}

fn read_binding(path: &Path) -> Result<RecoveryBinding> {
    const MAX_RECORD_BYTES: u64 = 1024 * 1024;

    let file =
        File::open(path).map_err(|error| Error::io("Abrir registro de implantação", error))?;
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| Error::io("Ler registro de implantação", error))?;
    if bytes.len() as u64 > MAX_RECORD_BYTES {
        return Err(Error::new(
            ErrorKind::Validation,
            "Registro de implantação grande demais",
        ));
    }

    let record: DeploymentRecord = serde_json::from_slice(&bytes)
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;
    if record.schema_version != 1
        || record.execution_state != "awaiting_first_boot"
        || !record.errors.is_empty()
        || record.restore.recovery_operation != record.operation_id
        || record.restore.deployment_plan_sha256 != record.plan_fingerprint
    {
        return Err(Error::new(
            ErrorKind::Integrity,
            "Registro de implantação incompleto ou inconsistente",
        ));
    }

    Ok(record.restore)
}
