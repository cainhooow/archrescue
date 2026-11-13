//! Order and labels from the recovery design, with an additional integrity page.

use super::*;

pub const ORDER: [usize; 10] = [
    DASHBOARD,
    DIAGNOSIS,
    INTEGRITY_PAGE,
    BACKUP,
    REPAIR_PAGE,
    VERIFY_PAGE,
    REPORTS,
    deployment::DEPLOY,
    deployment::RESTORE,
    console::PAGE,
];

pub fn owner(section: usize) -> usize {
    if section == DISKS {
        DASHBOARD
    } else {
        section
    }
}

pub fn label(section: usize, compact: bool, technician: bool) -> &'static str {
    match (section, compact) {
        (DASHBOARD | DISKS, _) => "Detectar",
        (DIAGNOSIS, _) => "Diagnosticar",
        (INTEGRITY_PAGE, true) => "Integridade",
        (INTEGRITY_PAGE, false) => "Integridade do Windows",
        (BACKUP, false) if technician => "Backup e preservação",
        (BACKUP, true) => "Salvar arquivos",
        (BACKUP, false) => "Salvar meus arquivos",
        (REPAIR_PAGE, _) => "Reparar",
        (VERIFY_PAGE, _) => "Verificar",
        (REPORTS, _) => "Relatar",
        (deployment::DEPLOY, true) => "Implantar",
        (deployment::DEPLOY, false) => "Implantar Windows",
        (deployment::RESTORE, true) => "Restaurar",
        (deployment::RESTORE, false) => "Restaurar arquivos",
        (console::PAGE, _) => "Console",
        _ => "",
    }
}

pub fn context_offset(section: usize) -> i32 {
    match section {
        DASHBOARD | DISKS | REPAIR_PAGE | VERIFY_PAGE | REPORTS => 68,
        DIAGNOSIS | BACKUP => 98,
        deployment::DEPLOY | deployment::RESTORE => 156,
        _ => 196,
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
            "Escolha uma operação e confirme a pasta original para retomar.",
        ),
        BACKUP => (
            if app.technician {
                "Backup e preservação de dados"
            } else {
                "Salvar meus arquivos"
            },
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
        console::PAGE => (
            "Console de diagnóstico",
            "Execute uma ação de diagnóstico e consulte a saída nesta sessão.",
        ),
        _ => (
            "Restaurar arquivos após reinstalar",
            "Verifique o backup e exporte uma referência para usar no Windows instalado.",
        ),
    }
}
