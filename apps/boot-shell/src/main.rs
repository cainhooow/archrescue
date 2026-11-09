//! WinPE shell entry point and a portable, executable diagnostic CLI.

#[cfg(windows)]
mod gui;
mod i18n;
#[cfg(windows)]
mod views;

use std::{ffi::OsString, path::PathBuf, sync::atomic::AtomicBool};

use archrescue_core::{Error, ErrorKind, Result, State, BUILD_ID, VERSION};

fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| Error::new(ErrorKind::Validation, e.to_string()))?;

    println!("{json}");
    Ok(())
}

fn paths(args: &[OsString]) -> Vec<PathBuf> {
    args.iter().map(PathBuf::from).collect()
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
