use std::{ffi::OsString, path::Path, sync::atomic::AtomicBool};

use archrescue_core::{Error, ErrorKind, Result};
use archrescue_integrity::{IntegrityReport, ScanMode};

pub fn command(command: &str, args: &[OsString]) -> Result<()> {
    let cancel = AtomicBool::new(false);
    match (command, args) {
        ("integrity", [root]) => super::print_json(&archrescue_integrity::scan(
            Path::new(root),
            ScanMode::PassiveOffline,
            &cancel,
            |_| {},
        )),
        ("integrity", [root, flag]) if flag == "--native" => {
            super::print_json(&archrescue_integrity::scan(
                Path::new(root),
                ScanMode::NativeVerification,
                &cancel,
                |_| {},
            ))
        }
        ("integrity-report", [root, destination, reference @ ..])
            if reference.is_empty() || reference.len() == 2 =>
        {
            let root = Path::new(root);
            let mut report = archrescue_evidence::Report::collect(&[root.to_owned()]);
            let mut scan =
                archrescue_integrity::scan(root, ScanMode::PassiveOffline, &cancel, |_| {});

            if let [media, index] = reference {
                let index = index
                    .to_str()
                    .and_then(|index| index.parse::<u32>().ok())
                    .ok_or_else(|| Error::new(ErrorKind::Validation, "Índice da mídia inválido"))?;
                let media =
                    archrescue_deployment::inspect(Path::new(media), None, &cancel, |_, _| {})?;
                let image = media
                    .images
                    .iter()
                    .find(|image| image.index == index)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::Validation,
                            "Índice ausente na mídia inspecionada",
                        )
                    })?;
                add_reference(&mut scan, &media, image);
            }

            report.integrity.push(scan);
            let output = archrescue_evidence::export(&report, Path::new(destination))?;
            println!("Relatório verificado: {}", output.display());
            Ok(())
        }
        _ => Err(Error::new(
            ErrorKind::Validation,
            "Comando de integridade inválido; use --help",
        )),
    }
}

pub fn add_reference(
    report: &mut IntegrityReport,
    media: &archrescue_deployment::Media,
    image: &archrescue_deployment::Image,
) {
    use archrescue_integrity::{ReferenceAssessment, ReferenceStatus};

    let incompatible = report
        .windows
        .as_ref()
        .zip(image.architecture.as_ref())
        .is_some_and(|(windows, machine)| !windows.architecture.eq_ignore_ascii_case(machine));
    let (status, reason) = if incompatible {
        (
            ReferenceStatus::Unsupported,
            "Arquitetura do índice diverge da arquitetura observada.",
        )
    } else {
        (ReferenceStatus::Inconclusive, "O índice foi inspecionado. A versão dos arquivos, a edição, o idioma, os recursos e o nível de manutenção não puderam ser equiparados.")
    };

    report.reference = Some(ReferenceAssessment {
        path: media.path.clone(),
        sha256: media.sha256.clone(),
        index: image.index,
        architecture: image.architecture.clone(),
        edition: image.edition.clone(),
        version: image.version.clone(),
        status,
        reason: reason.into(),
    });
}
