//! Presentation of actual report state in the primary locale.

use std::fmt::Write;

use archrescue_core::State;
use archrescue_diagnostics::recovery::Confidence;
use archrescue_evidence::Report;

pub fn section(section: usize, report: &Report, technician: bool) -> String {
    let mut text = match section {
        0 => detect(report, technician),
        1 => diagnose(report, technician),
        2 => String::new(),
        3 => repair(report, technician),
        4 => verify(report, technician),
        5 => session(report),
        _ => String::new(),
    };
    if technician && section == 5 {
        text.push_str("\n\n");
        text.push_str(&report.readable());
    }

    text
}

fn detect(report: &Report, technician: bool) -> String {
    let mut text = String::from("DETECTAR INSTALAÇÕES E DISCOS\n\n");
    for installation in &report.installations.installations {
        let _ = writeln!(
            text,
            "{}\nKernel: {} | Carregador UEFI: {} | Versão do kernel: {}\n",
            installation.path.display(),
            installation
                .architecture
                .as_deref()
                .unwrap_or("desconhecido"),
            installation
                .loader_architecture
                .as_deref()
                .unwrap_or("desconhecido"),
            installation.version.as_deref().unwrap_or("desconhecida"),
        );
    }
    if report.installations.installations.is_empty() {
        text.push_str("Nenhum Windows offline corroborado nas raízes consultadas.\n\n");
    }

    for disk in &report.inventory.disks {
        let _ = writeln!(
            text,
            "Disco {:?} | {} | {}",
            disk.number,
            disk.style.as_deref().unwrap_or("estilo indisponível"),
            disk.identity
        );
        for partition in &disk.partitions {
            let _ = writeln!(
                text,
                "  Partição {} | {} bytes | {}",
                partition.number,
                partition.length,
                if partition.efi {
                    "ESP"
                } else {
                    &partition.kind
                },
            );
            if technician {
                let _ = writeln!(
                    text,
                    "    GUID {:?} | Deslocamento {}",
                    partition.id, partition.offset
                );
            }
        }
        if technician {
            let _ = writeln!(
                text,
                "  Caminho {} | Erros {:?}",
                disk.device_path, disk.errors
            );
        }
    }
    for volume in &report.inventory.volumes {
        let paths = volume
            .paths
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            text,
            "\n{} | {} | {}",
            if paths.is_empty() {
                "Sem caminho de acesso"
            } else {
                &paths
            },
            volume
                .filesystem
                .as_deref()
                .unwrap_or("sistema de arquivos indisponível"),
            if volume.space.is_some() {
                "espaço consultado"
            } else {
                "espaço indisponível"
            },
        );
        if technician {
            let _ = writeln!(
                text,
                "  GUID {} | Discos {:?} | Erros {:?}",
                volume.identity, volume.physical_disks, volume.errors
            );
        }
    }

    text
}

fn diagnose(report: &Report, technician: bool) -> String {
    let mut text = String::from("DIAGNOSTICAR\n\n");
    for finding in &report.diagnosis.findings {
        let label = match finding.confidence {
            Confidence::Observed => "Observado",
            Confidence::ConfirmedFailure => "Falha confirmada",
            Confidence::SuspectedFailure => "Falha suspeita",
            Confidence::MissingEvidence => "Evidência indisponível",
            Confidence::Unsupported => "Configuração não suportada",
        };
        let _ = writeln!(
            text,
            "{label} | {}\n{}\n{}\n",
            finding.affected.display(),
            finding.evidence,
            finding.next_action
        );
        if technician {
            let _ = writeln!(
                text,
                "{} | {} | bloqueio de reparo: {}\n",
                finding.id, finding.severity, finding.blocks_repair
            );
        }
    }
    if technician {
        for association in &report.diagnosis.associations {
            let _ = writeln!(text, "Associação: {association:#?}");
        }
        for error in report
            .inventory
            .errors
            .iter()
            .chain(&report.installations.errors)
            .chain(&report.boot.errors)
        {
            let _ = writeln!(text, "{error} | Código {:?}", error.os_code);
        }
    }

    text
}

fn repair(report: &Report, technician: bool) -> String {
    let mut text = String::from(
        "REPARAR BOOT UEFI\n\nInforme a raiz da ESP e um destino independente. A execução do BCDBoot é reservada ao clone registrado e à autorização do plano no laboratório.\n",
    );
    let Some(plan) = report.repair_plans.last() else {
        return text;
    };

    let _ = writeln!(
        text,
        "\nWindows: {}\nESP: {}\nDestino de preservação: {}\nEspaço temporário: {} bytes\n",
        plan.windows_root.display(),
        plan.efi_root.display(),
        plan.backup_destination.display(),
        plan.scratch_estimate_bytes,
    );
    if let Some(target) = &plan.target {
        let _ = writeln!(
            text,
            "Disco: {}\nPartição EFI: {}",
            target.disk_id, target.efi_partition_id
        );
    }
    for change in &plan.intended_changes {
        let _ = writeln!(text, "Alteração prevista: {change}");
    }
    for blocker in &plan.blockers {
        let _ = writeln!(text, "Bloqueio: {blocker}");
    }
    let _ = writeln!(text, "Recuperação: {}", plan.rollback);
    if technician {
        let _ = writeln!(
            text,
            "\nPlano {}\nArgumentos: {:?}\nAlvo: {:#?}",
            plan.id, plan.command, plan.target
        );
    }

    text
}

fn verify(report: &Report, technician: bool) -> String {
    let mut text = String::from("VERIFICAR RESULTADOS\n\n");
    let Some(backup) = report.transactions.last() else {
        text.push_str("Inicie ou retome um backup para obter um resultado verificável.");
        return text;
    };

    let _ = writeln!(
        text,
        "{} | {}\nPlanejados: {} bytes | Gravados: {} bytes | Verificados: {} bytes\nArquivos verificados: {} | Reutilizados após leitura: {}\nDestino: {}\n",
        backup.status.operation_id,
        state_label(backup.status.state),
        backup.progress.planned_bytes,
        backup.progress.acknowledged_bytes,
        backup.progress.verified_bytes,
        backup.progress.verified_files,
        backup.progress.skipped_files,
        backup.directory.display(),
    );
    if let Some(error) = &backup.status.error {
        let _ = writeln!(text, "{error}");
    }
    if technician {
        let _ = writeln!(
            text,
            "Origem: {:?}\nDestino: {:?}\nCheckpoint: {}\nTempo desta execução: {} s",
            backup.manifest.source_identity,
            backup.manifest.destination_identity,
            backup.manifest.plan_fingerprint,
            backup.progress.elapsed_seconds,
        );
        for (file, checkpoint) in backup.manifest.files.iter().zip(&backup.files) {
            let _ = writeln!(
                text,
                "{} | {:?} | {} bytes | {}",
                file.relative.display(),
                checkpoint.state,
                checkpoint.acknowledged_bytes,
                file.sha256
            );
        }
    }

    text
}

fn session(report: &Report) -> String {
    let mut text = String::from("RELATAR SESSÃO\n\nUse um destino persistente em disco separado para exportar JSON e texto.\n\n");
    let _ = writeln!(
        text,
        "{} achados | {} resultados de backup | {} planos de reparo | {} erros na sessão",
        report.diagnosis.findings.len(),
        report.transactions.len(),
        report.repair_plans.len(),
        report.session_errors.len(),
    );
    for decision in &report.decisions {
        let _ = writeln!(text, "{decision}");
    }

    text
}

pub fn state_label(state: State) -> &'static str {
    match state {
        State::Pending => "Pendente",
        State::Validating => "Validando",
        State::Running => "Em execução",
        State::Verifying => "Verificando",
        State::Completed => "Concluído e verificado",
        State::Failed => "Falhou — resultado incompleto",
        State::Cancelled => "Cancelado — resultado incompleto",
        State::Blocked => "Bloqueado",
    }
}
