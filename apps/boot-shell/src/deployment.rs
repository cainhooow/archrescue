use std::{ffi::OsString, fs, path::Path, sync::atomic::AtomicBool};

use archrescue_core::{Error, ErrorKind, Result};
use archrescue_deployment::{self as deployment, Intent, Plan, Qualification};

pub fn command(command: &str, args: &[OsString]) -> Result<()> {
    let cancel = AtomicBool::new(false);
    let invalid = || {
        Error::new(
            ErrorKind::Validation,
            "Argumentos inválidos. Consulte --help",
        )
    };
    let string = |index: usize| args[index].to_str().ok_or_else(invalid);

    match command {
        "deployment-media" if matches!(args.len(), 1 | 2) => {
            let media = deployment::inspect(
                Path::new(&args[0]),
                args.get(1).and_then(|value| value.to_str()),
                &cancel,
                |read, total| {
                    if read == total {
                        eprintln!("Imagem lida: {read}/{total} bytes");
                    }
                },
            )?;
            super::print_json(&media)
        }
        "deployment-plan" if matches!(args.len(), 6 | 7) => {
            let plan = deployment::create_plan(
                Path::new(&args[0]),
                string(1)?,
                string(2)?.parse().map_err(|_| invalid())?,
                string(3)?,
                Intent::parse(string(4)?)?,
                args.get(6).map(Path::new),
                &cancel,
            )?;
            let output = deployment::export(&plan, Path::new(&args[5]))?;
            eprintln!("Plano exportado: {}", output.display());
            super::print_json(&plan)?;

            if plan.result != Qualification::Supported {
                return Err(Error::new(ErrorKind::Incomplete, plan.next_action));
            }

            Ok(())
        }
        "deployment-authorize" if args.len() == 2 => {
            let path = archrescue_storage::safe_existing(Path::new(&args[0]))?;
            let bytes = fs::read(path).map_err(|error| Error::io("Ler plano", error))?;
            let plan: Plan = serde_json::from_slice(&bytes)
                .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))?;

            plan.validate_authorization(string(1)?, &cancel)?;
            println!("Confirmação validada. Execução de implantação indisponível nesta versão.");
            Ok(())
        }
        _ => Err(invalid()),
    }
}
