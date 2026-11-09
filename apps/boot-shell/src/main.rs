//! WinPE shell entry point and a portable, executable diagnostic CLI.
#[cfg(windows)]
mod gui;
mod i18n;
use archrescue_core::{Error, ErrorKind, Result, State, VERSION};
use std::{path::PathBuf, sync::atomic::AtomicBool};
fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .map_err(|e| Error::new(ErrorKind::Validation, e.to_string()))?
    );
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{}: {e}", i18n::FAILED);
        std::process::exit(1);
    }
}
fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() {
        #[cfg(windows)]
        {
            return gui::run();
        }
        #[cfg(not(windows))]
        {
            println!("ArchRescue {VERSION}\nInterface gráfica disponível no Windows. Use --help para os comandos de diagnóstico.");
            return Ok(());
        }
    }
    let command = args[0]
        .to_str()
        .ok_or_else(|| Error::new(ErrorKind::Validation, "Invalid command encoding"))?;
    match command {
        "--version" if args.len()==1=>println!("ArchRescue {VERSION} {} {}",std::env::consts::OS,std::env::consts::ARCH),
        "--help" if args.len()==1=>println!("ArchRescue {VERSION}\nComandos (caminhos absolutos):\n  inventory\n  discover RAIZ...\n  boot RAIZ...\n  browse DIRETÓRIO\n  backup ORIGEM DESTINO SELEÇÃO_RELATIVA...\n  report DESTINO [RAIZ...]\n\nBackup: rejeita o mesmo volume/disco, links e conflitos; copia apenas conteúdo e verifica SHA-256.\nRelatórios são locais. Exportar para RAM não preserva dados após reiniciar."),
        "inventory" if args.len()==1=>print_json(&archrescue_storage::inventory())?,
        "discover" if args.len()>1=>print_json(&archrescue_windows_detection::discover(&args[1..].iter().map(PathBuf::from).collect::<Vec<_>>()))?,
        "boot" if args.len()>1=>print_json(&archrescue_diagnostics::inspect(&args[1..].iter().map(PathBuf::from).collect::<Vec<_>>()))?,
        "browse" if args.len()==2=>print_json(&archrescue_backup::browse(&PathBuf::from(&args[1]))?)?,
        "backup" if args.len()>3=>{
            let cancel=AtomicBool::new(false);
            let result=archrescue_backup::recover(&PathBuf::from(&args[1]),&args[3..].iter().map(PathBuf::from).collect::<Vec<_>>(),&PathBuf::from(&args[2]),&cancel,|bytes,total|eprintln!("Copiando: {bytes}/{total} bytes"));
            print_json(&result)?;
            if result.operation.state!=State::Completed{return Err(Error::new(ErrorKind::Incomplete,format!("Operação {}: {:?}",result.operation.id,result.operation.state)));}
        },
        "report" if args.len()>1=>{let report=archrescue_evidence::Report::collect(&args[2..].iter().map(PathBuf::from).collect::<Vec<_>>());let output=archrescue_evidence::export(&report,&PathBuf::from(&args[1]))?;println!("Relatório verificado: {}",output.display());},
        _=>return Err(Error::new(ErrorKind::Validation,"Comando ou argumentos inválidos; use --help"))
    }
    Ok(())
}
