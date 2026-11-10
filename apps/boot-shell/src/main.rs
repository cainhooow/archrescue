//! WinPE shell entry point and a portable, executable diagnostic CLI.

#[cfg(windows)]
mod gui;
mod i18n;
#[cfg(windows)]
mod views;

use std::{ffi::OsString, path::PathBuf, sync::atomic::AtomicBool};

use archrescue_backup::transaction::{self, TransactionReport};
use archrescue_core::{Error, ErrorKind, Result, State, BUILD_ID, VERSION};
use archrescue_evidence::repair;

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| Error::new(ErrorKind::Validation, e.to_string()))?;

    println!("{json}");
    Ok(())
}

fn paths(args: &[OsString]) -> Vec<PathBuf> {
    args.iter().map(PathBuf::from).collect()
}

fn transaction_result(report: &TransactionReport) -> Result<()> {
    print_json(report)?;

    match &report.status.error {
        Some(error) => Err(error.clone()),
        None => Ok(()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}: {error}", i18n::FAILED);
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();

    if args.is_empty() {
        #[cfg(windows)]
        return gui::run();
        #[cfg(not(windows))]
        {
            println!("ArchRescue {VERSION}\nInterface gráfica disponível no Windows. Use --help.");
            return Ok(());
        }
    }

    let command = args[0]
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::Validation, "Invalid command encoding"))?;

    match command {
        "--version" if args.len() == 1 => {
            println!(
                "ArchRescue {VERSION} {} {} build={BUILD_ID}",
                std::env::consts::OS,
                std::env::consts::ARCH
            );
        }
        "--help" if args.len() == 1 => {
            println!("ArchRescue {VERSION}\n{}", i18n::HELP);
        }
        "inventory" if args.len() == 1 => print_json(&archrescue_storage::inventory())?,
        "discover" if args.len() > 1 => {
            let discovery = archrescue_windows_detection::discover(&paths(&args[1..]));
            print_json(&discovery)?;
        }
        "boot" if args.len() > 1 => {
            let report = archrescue_diagnostics::inspect(&paths(&args[1..]));
            print_json(&report)?;
        }
        "diagnose" if args.len() > 1 => {
            print_json(&archrescue_evidence::Report::collect(&paths(&args[1..])))?;
        }
        "repair-plan" if matches!(args.len(), 4 | 6) => {
            let paths = paths(&args[1..]);
            let plan = repair::plan(
                &paths[0],
                &paths[1],
                &paths[2],
                paths.get(3).map(PathBuf::as_path),
                paths.get(4).map(PathBuf::as_path),
            )?;
            print_json(&plan)?;
        }
        "repair-lab" if args.len() == 3 => {
            let path = archrescue_storage::safe_existing(&PathBuf::from(&args[1]))?;
            let bytes =
                std::fs::read(path).map_err(|error| Error::io("Read repair plan", error))?;
            let plan = serde_json::from_slice(&bytes)
                .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;
            let authorization = args[2].to_str().unwrap_or_default();
            let result = repair::execute(&plan, authorization)?;
            print_json(&result)?;
            if !result.postconditions_met {
                return Err(Error::new(
                    ErrorKind::Incomplete,
                    "Verificação após o reparo falhou",
                ));
            }
        }
        "browse" if args.len() == 2 => {
            let directory = PathBuf::from(&args[1]);
            let entries = archrescue_backup::browse(&directory)?;
            print_json(&entries)?;
        }
        "backup" if args.len() > 3 => {
            let cancel = AtomicBool::new(false);
            let source = PathBuf::from(&args[1]);
            let selections = paths(&args[3..]);
            let destination = PathBuf::from(&args[2]);

            let result = archrescue_backup::recover(
                &source,
                &selections,
                &destination,
                &cancel,
                |bytes, total| eprintln!("Copiando: {bytes}/{total} bytes"),
            );
            print_json(&result)?;

            if result.operation.state != State::Completed {
                return Err(Error::new(
                    ErrorKind::Incomplete,
                    format!(
                        "Operação {}: {:?}",
                        result.operation.id, result.operation.state
                    ),
                ));
            }
        }
        "backup-start" if args.len() > 3 => {
            let cancel = AtomicBool::new(false);
            let source = PathBuf::from(&args[1]);
            let destination = PathBuf::from(&args[2]);
            let directory =
                transaction::prepare(&source, &paths(&args[3..]), &destination, &cancel)?;
            eprintln!("Checkpoint: {}", directory.display());

            let report = transaction::resume(&source, &directory, &cancel, transaction_progress)?;
            transaction_result(&report)?;
        }
        "backup-list" if args.len() == 2 => {
            print_json(&transaction::checkpoints(&PathBuf::from(&args[1]))?)?;
        }
        "backup-resume" | "backup-verify" if args.len() == 3 => {
            let cancel = AtomicBool::new(false);
            let source = PathBuf::from(&args[1]);
            let directory = PathBuf::from(&args[2]);
            let report = if command == "backup-resume" {
                transaction::resume(&source, &directory, &cancel, transaction_progress)?
            } else {
                transaction::verify(&source, &directory, &cancel)?
            };

            transaction_result(&report)?;
        }
        "report" if args.len() > 1 => {
            let report = archrescue_evidence::Report::collect(&paths(&args[2..]));
            let output = archrescue_evidence::export(&report, &PathBuf::from(&args[1]))?;

            println!("Relatório verificado: {}", output.display());
        }
        _ => {
            return Err(Error::new(
                ErrorKind::Validation,
                "Comando ou argumentos inválidos; use --help",
            ))
        }
    }

    Ok(())
}

fn transaction_progress(progress: &transaction::Progress) {
    if progress.acknowledged_bytes.is_multiple_of(16 * 1024 * 1024)
        || progress.verified_bytes == progress.planned_bytes
    {
        eprintln!(
            "{} | {} | gravados {}/{} bytes | verificados {} bytes",
            progress.operation_id,
            progress.stage,
            progress.acknowledged_bytes,
            progress.planned_bytes,
            progress.verified_bytes
        );
    }
}
