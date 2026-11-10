//! Primary locale: pt-BR. Keys are centralized for future localization.

pub const FAILED: &str = "Falha";

#[cfg(windows)]
pub const SECTIONS: [&str; 6] = [
    "Detectar",
    "Diagnosticar",
    "Proteger dados",
    "Reparar",
    "Verificar",
    "Relatar",
];

#[cfg(windows)]
pub const LOADING: &str = "Carregando dados reais…";

#[cfg(windows)]
pub const EMPTY: &str =
    "Nenhum resultado disponível. Atualize os dados ou selecione uma pasta acessível.";

pub const HELP: &str = "Comandos (caminhos absolutos):
  inventory
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
