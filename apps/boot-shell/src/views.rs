//! Presentation of actual report state in the primary locale.

use archrescue_core::{State, VERSION};
use archrescue_evidence::Report;

pub fn section(section: usize, report: &Report) -> String {
    match section {
        0 => dashboard(report),
        1 => storage(report),
        2 => installations(report),
        3 => boot(report),
        5 => backups(report),
        6 => format!(
            "RELATÓRIOS\nInforme abaixo um diretório persistente autorizado e use Exportar relatório.\n\
             Dados em X: (RAM) não sobrevivem à reinicialização.\n\n{}",
            report.readable(),
        ),
        _ => String::new(),
    }
}

fn dashboard(report: &Report) -> String {
    let errors = report
        .inventory
        .errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "ARCHRESCUE {VERSION}\nRecuperação e diagnóstico • ArchGTi\n\n\
         {} discos | {} volumes | {} candidatos Windows\n\n\
         Origem: somente leitura pela aplicação.\n\
         Backup: conteúdo verificado com SHA-256 em outro disco.\n\n\
         No Explorador, informe uma pasta absoluta, selecione com Ctrl/Shift\n\
         e informe o destino abaixo.\n\n\
         Não há diagnóstico de saúde física. \
         Partições inacessíveis são relatadas sem desbloqueio.\n\n{errors}",
        report.inventory.disks.len(),
        report.inventory.volumes.len(),
        report.installations.installations.len(),
    )
}

fn storage(report: &Report) -> String {
    let mut text = String::from("DISCOS E PARTIÇÕES\n\n");

    for disk in &report.inventory.disks {
        text.push_str(&format!(
            "{}\nDispositivo: {}\n\
             Capacidade: {:?} bytes | Barramento: {:?} | Removível: {:?}\nEstilo: {:?}\n",
            disk.identity, disk.device_path, disk.capacity, disk.bus, disk.removable, disk.style,
        ));

        for partition in &disk.partitions {
            text.push_str(&format!(
                "  Partição {}: {} bytes | {} | EFI: {}\n",
                partition.number, partition.length, partition.kind, partition.efi,
            ));
        }

        for error in &disk.errors {
            text.push_str(&format!("  Indisponível: {error}\n"));
        }
    }

    for volume in &report.inventory.volumes {
        text.push_str(&format!(
            "\nVolume: {}\nCaminhos: {:?} | Sistema: {:?}\n",
            volume.identity, volume.paths, volume.filesystem
        ));

        if let Some(space) = &volume.space {
            text.push_str(&format!(
                "Total: {} | Usado: {} | Disponível: {} bytes\n",
                space.total, space.used, space.available
            ));
        }

        for error in &volume.errors {
            text.push_str(&format!("Indisponível: {error}\n"));
        }
    }

    if report.inventory.disks.is_empty() {
        text.push_str("Nenhum disco enumerado. Consulte os erros no relatório.\n");
    }

    text
}

fn installations(report: &Report) -> String {
    let mut text = String::from(
        "INSTALAÇÕES WINDOWS\nCandidatos offline; não implica instalação íntegra.\n\n",
    );

    for install in &report.installations.installations {
        text.push_str(&format!(
            "{}\nArquitetura: {:?} | Versão: {:?}\nEvidências: {:?}\nLimitações: {:?}\n\n",
            install.path.display(),
            install.architecture,
            install.version,
            install.evidence,
            install.limitations
        ));
    }

    if report.installations.installations.is_empty() {
        text.push_str("Nenhuma instalação encontrada nas raízes acessíveis.\n");
    }

    for error in &report.installations.errors {
        text.push_str(&format!("Falha: {error}\n"));
    }

    text
}

fn boot(report: &Report) -> String {
    let mut text = String::from("DIAGNÓSTICO DE BOOT • SOMENTE LEITURA\n\n");

    for observation in &report.boot.observations {
        text.push_str(&format!(
            "{}\n{}: {}\n\n",
            observation.path.display(),
            observation.status,
            observation.detail
        ));
    }

    text.push_str(&report.boot.limitations.join("\n"));

    text
}

fn backups(report: &Report) -> String {
    let mut text = String::from(
        "BACKUP\nSelecione arquivos no Explorador e use Copiar seleção.\n\
        Conflitos bloqueiam a cópia; arquivos parciais permanecem não verificados.\n\n",
    );

    for backup in &report.operations {
        let verified_files = backup.files.iter().filter(|file| file.verified).count();

        text.push_str(&format!(
            "{}\nEstado: {}\n{} / {} bytes | {} / {} arquivos verificados\n",
            backup.operation.id,
            state_label(backup.operation.state),
            backup.copied_bytes,
            backup.planned_bytes,
            verified_files,
            backup.files.len()
        ));

        for error in &backup.operation.errors {
            text.push_str(&format!("Falha: {error}\n"));
        }
    }

    text
}

pub fn state_label(state: State) -> &'static str {
    match state {
        State::Pending => "Pendente",
        State::Validating => "Validando",
        State::Running => "Copiando",
        State::Verifying => "Verificando",
        State::Completed => "Concluído e verificado",
        State::Failed => "Falhou — resultado incompleto",
        State::Cancelled => "Cancelado — resultado incompleto",
        State::Blocked => "Bloqueado",
    }
}
