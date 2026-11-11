//! Primary locale: pt-BR. Keys are centralized for future localization.

pub const FAILED: &str = "Falha";

#[cfg(windows)]
pub const SECTIONS: [&str; 9] = [
    "Visão geral",
    "Diagnóstico",
    "Integridade do Windows",
    "Discos e partições",
    "Backup e recuperação",
    "Reparar inicialização",
    "Implantar Windows",
    "Restaurar arquivos",
    "Relatórios",
];

#[cfg(windows)]
pub const COMPACT_SECTIONS: [&str; 9] = [
    "Visão geral",
    "Diagnóstico",
    "Integridade",
    "Discos",
    "Backup",
    "Inicialização",
    "Implantação",
    "Restauração",
    "Relatórios",
];

#[cfg(windows)]
pub const LOADING: &str = "Carregando dados reais…";

#[cfg(windows)]
pub const EMPTY: &str =
    "Nenhum resultado disponível. Atualize os dados ou selecione uma pasta acessível.";

pub const HELP: &str = "Comandos (caminhos absolutos):
  inventory
  integrity RAIZ_WINDOWS [--native]
  integrity-report RAIZ_WINDOWS DESTINO [MÍDIA ÍNDICE]
  deployment-media IMAGEM_OU_RAIZ [SHA256_ESPERADO]
  deployment-plan IMAGEM SHA256 ÍNDICE ID_GPT MODO DESTINO [BACKUP]
  deployment-authorize PLANO_JSON AUTORIZAR-IMPLANTACAO:SHA256
  discover RAIZ...
  boot RAIZ...
  diagnose RAIZ...
  browse DIRETÓRIO
  backup ORIGEM DESTINO SELEÇÃO_RELATIVA...
  backup-start ORIGEM DESTINO SELEÇÃO_RELATIVA...
  backup-list DESTINO
  backup-resume ORIGEM DIRETÓRIO_DA_OPERAÇÃO
  backup-verify ORIGEM DIRETÓRIO_DA_OPERAÇÃO
  repair-plan RAIZ_WINDOWS RAIZ_ESP DESTINO [BACKUP_DADOS BACKUP_EFI]
  repair-lab PLANO_JSON AUTORIZO-ID_DO_PLANO
  report DESTINO [RAIZ...]

Backup: rejeita o mesmo volume/disco, links e conflitos; copia apenas conteúdo e verifica SHA-256.
Relatórios são locais. Exportar para RAM não preserva dados após reiniciar.";
