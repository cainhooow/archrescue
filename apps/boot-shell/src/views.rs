//! Presentation of actual report state in the primary locale.

use std::fmt::Write;

use archrescue_core::State;
use archrescue_evidence::Report;

pub fn repair(plan: Option<&archrescue_evidence::repair::RepairPlan>, technician: bool) -> String {
    let mut text = String::from(
        "REPARAR BOOT UEFI\n\nConfira a partição EFI sugerida e escolha onde preservar os arquivos. A execução do BCDBoot é reservada ao clone registrado e à autorização do plano no laboratório.\n",
    );
    let Some(plan) = plan else {
        return text;
    };

    let _ = writeln!(
        text,
        "\nWindows: {}\nESP: {}\nDestino de preservação: {}\nEspaço temporário: {} bytes\n",
        crate::paths::display(&plan.windows_root, technician),
        crate::paths::display(&plan.efi_root, technician),
        crate::paths::display(&plan.backup_destination, technician),
        plan.scratch_estimate_bytes,
    );
    if let Some(target) = plan.target.as_ref().filter(|_| technician) {
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

pub fn verify(report: &Report, technician: bool) -> String {
    let mut text = String::from("");
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
        crate::paths::display(&backup.directory, technician),
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

pub fn session(report: &Report, technician: bool) -> String {
    let mut text = if report.decisions.is_empty() {
        "Nenhuma decisão registrada nesta sessão.".into()
    } else {
        report.decisions.join("\n\n")
    };

    if technician {
        text.push_str("\n\n");
        text.push_str(&report.readable());
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
