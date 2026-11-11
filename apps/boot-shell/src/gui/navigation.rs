//! Order and labels from the recovery design, with an additional integrity page.

use super::*;

pub const ORDER: [usize; 9] = [
    DASHBOARD,
    DIAGNOSIS,
    INTEGRITY_PAGE,
    BACKUP,
    REPAIR_PAGE,
    VERIFY_PAGE,
    REPORTS,
    deployment::DEPLOY,
    deployment::RESTORE,
];

pub fn owner(section: usize) -> usize {
    if section == DISKS {
        DASHBOARD
    } else {
        section
    }
}

pub fn label(section: usize, compact: bool) -> &'static str {
    match (section, compact) {
        (DASHBOARD | DISKS, _) => "Detectar",
        (DIAGNOSIS, _) => "Diagnosticar",
        (INTEGRITY_PAGE, true) => "Integridade",
        (INTEGRITY_PAGE, false) => "Integridade do Windows",
        (BACKUP, _) => "Proteger dados",
        (REPAIR_PAGE, _) => "Reparar",
        (VERIFY_PAGE, _) => "Verificar",
        (REPORTS, _) => "Relatar",
        (deployment::DEPLOY, true) => "Implantar",
        (deployment::DEPLOY, false) => "Implantar Windows",
        (deployment::RESTORE, true) => "Restaurar",
        (deployment::RESTORE, false) => "Restaurar arquivos",
        _ => "",
    }
}

pub fn adjacent(section: usize, offset: isize) -> Option<usize> {
    let index = ORDER.iter().position(|&page| page == owner(section))?;
    ORDER.get(index.checked_add_signed(offset)?).copied()
}

pub fn heading(app: &App) -> (&'static str, &'static str) {
    match app.section {
        DASHBOARD => (
            "Detectar instalações e discos",
            "Confira o Windows offline, os discos físicos e os volumes acessíveis.",
        ),
        DISKS => (
            "Discos e partições",
            "Consulte as partições, volumes e identidades observadas.",
        ),
        DIAGNOSIS => (
            "Diagnosticar inicialização",
            "Veja o que foi observado, as evidências disponíveis e o próximo passo.",
        ),
        INTEGRITY_PAGE => (
            "Integridade do Windows",
            "Analise executáveis e metadados da instalação offline selecionada.",
        ),
        BACKUP if app.preview.is_some() => (
            "Revisar plano de backup",
            "Confira a origem, o destino e o conteúdo selecionado antes de confirmar.",
        ),
        BACKUP if app.checkpoints.is_some() => (
            "Checkpoints de backup",
            "Escolha uma operação no destino e informe a origem para retomar.",
        ),
        BACKUP => (
            "Proteger dados",
            "Selecione arquivos e pastas para copiar o conteúdo a um disco separado.",
        ),
        REPAIR_PAGE => (
            "Reparar boot UEFI",
            "Gere uma prévia para conferir a partição EFI, os requisitos e os bloqueios.",
        ),
        VERIFY_PAGE => (
            "Verificar resultados",
            "Consulte o resultado mais recente e releia o backup quando necessário.",
        ),
        REPORTS => (
            "Relatar sessão",
            "Exporte resultados, decisões e erros para consultar depois de reiniciar.",
        ),
        deployment::DEPLOY => (
            "Planejar implantação",
            "Inspecione uma imagem, selecione o alvo GPT e revise o plano de implantação.",
        ),
        _ => (
            "Restaurar arquivos após reinstalar",
            "Verifique o backup e exporte uma referência para usar no Windows instalado.",
        ),
    }
}
