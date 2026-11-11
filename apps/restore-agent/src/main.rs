use std::{path::PathBuf, sync::atomic::AtomicBool};

use archrescue_core::{Error, ErrorKind, Result, BUILD_ID, VERSION};
use archrescue_restore::{ConflictPolicy, Folder, Mapping, Profile, Request};

const HELP: &str = "Agente de Restauração ArchRescue
  --version
  perfil
  previa BACKUP SID POLÍTICA PASTA_ORIGEM PASTA_CONHECIDA [...]
  restaurar BACKUP SID POLÍTICA AUTORIZAR-RESTAURACAO:SHA256 PASTA_ORIGEM PASTA_CONHECIDA [...]

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
        "previa" | "restaurar" => {
            let executing = command == "restaurar";
            let start = if executing { 5 } else { 4 };
            if args.len() < start + 2 || (args.len() - start) % 2 != 0 {
                return Err(invalid());
            }
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
                let report = archrescue_restore::execute(
                    &request,
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
                print_json(&archrescue_restore::preview(&request, &cancel)?)?;
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
