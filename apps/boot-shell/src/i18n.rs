//! Primary locale: pt-BR. Keys are centralized for future localization.
pub const FAILED: &str = "Falha";
#[cfg(windows)]
pub const SECTIONS: [&str; 7] = [
    "Painel",
    "Discos e Partições",
    "Instalações Windows",
    "Diagnóstico de Boot",
    "Explorador de Arquivos",
    "Backup",
    "Relatórios",
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
  browse DIRETÓRIO
  backup ORIGEM DESTINO SELEÇÃO_RELATIVA...
  report DESTINO [RAIZ...]

Backup: rejeita o mesmo volume/disco, links e conflitos; copia apenas conteúdo e verifica SHA-256.
Relatórios são locais. Exportar para RAM não preserva dados após reiniciar.";
