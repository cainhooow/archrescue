//! Bounded, passive inspection of quiescent offline Windows volumes.

mod files;
mod servicing;

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use archrescue_core::{Error, ErrorKind, Result, BUILD_ID};
use archrescue_storage::{
    identity, recovery_location, safe_existing, RecoveryLocation, VolumeIdentity,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    FindingsDetected,
    NoDetectedIssue,
    Inconclusive,
    Unsupported,
    Blocked,
    Cancelled,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Self::FindingsDetected => "Achados detectados",
            Self::NoDetectedIssue => "Sem achados nas verificações executadas",
            Self::Inconclusive => "Diagnóstico inconclusivo",
            Self::Unsupported => "Configuração fora do perfil",
            Self::Blocked => "Análise bloqueada",
            Self::Cancelled => "Análise cancelada",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanMode {
    PassiveOffline,
    NativeVerification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Boot,
    System,
    ComponentStore,
    ServicingHistory,
}

impl Category {
    pub fn label(self) -> &'static str {
        match self {
            Self::Boot => "Inicialização",
            Self::System => "Arquivos do sistema",
            Self::ComponentStore => "Repositório de componentes",
            Self::ServicingHistory => "Histórico de manutenção",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Observation {
    VerifiedStructure,
    ConfirmedMissing,
    ConfirmedCorruption,
    SuspectedCorruption,
    HistoricalFailure,
    Inconclusive,
    Unsupported,
}

impl Observation {
    pub fn label(self) -> &'static str {
        match self {
            Self::VerifiedStructure => "Estrutura verificada",
            Self::ConfirmedMissing => "Ausência confirmada",
            Self::ConfirmedCorruption => "Estrutura inválida confirmada",
            Self::SuspectedCorruption => "Indício a confirmar",
            Self::HistoricalFailure => "Falha histórica",
            Self::Inconclusive => "Evidência inconclusiva",
            Self::Unsupported => "Verificação fora do perfil",
        }
    }

    fn actionable(self) -> bool {
        matches!(
            self,
            Self::ConfirmedMissing | Self::ConfirmedCorruption | Self::SuspectedCorruption
        )
    }

    fn severity(self) -> Severity {
        match self {
            Self::ConfirmedMissing | Self::ConfirmedCorruption => Severity::Error,
            Self::SuspectedCorruption | Self::Inconclusive => Severity::Warning,
            _ => Severity::Info,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub identifier: String,
    pub category: Category,
    pub path: PathBuf,
    pub expected_component: String,
    pub observation: Observation,
    pub severity: Severity,
    pub evidence: String,
    pub next_action: String,
    pub error: Option<Error>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowsIdentity {
    pub architecture: String,
    pub file_version: String,
    pub version_source: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub schema_version: u32,
    pub build_id: String,
    pub root: PathBuf,
    pub windows_directory: PathBuf,
    pub volume: Option<VolumeIdentity>,
    pub recovery_location: Option<RecoveryLocation>,
    pub windows: Option<WindowsIdentity>,
    pub mode: ScanMode,
    pub started_unix: u64,
    pub completed_unix: u64,
    pub status: Status,
    pub checks: Vec<Check>,
    pub limitations: Vec<String>,
    pub not_executed: Vec<String>,
    pub errors: Vec<Error>,
}

impl IntegrityReport {
    pub fn findings(&self) -> impl Iterator<Item = &Check> {
        self.checks
            .iter()
            .filter(|check| check.observation != Observation::VerifiedStructure)
    }

    pub fn recommendation(&self) -> &'static str {
        match self.status {
            Status::FindingsDetected => "Faça e verifique um backup independente. Revise os achados antes de preparar um plano de recuperação.",
            Status::NoDetectedIssue => "Confira a configuração de inicialização e preserve os dados antes de escolher uma ação.",
            Status::Inconclusive | Status::Cancelled => "Preserve os dados acessíveis. Complete a coleta de evidências antes de alterar o Windows.",
            Status::Unsupported | Status::Blocked => "Verifique a identidade, o acesso ao volume e o perfil de Windows suportado.",
        }
    }
}

pub struct Progress {
    pub phase: &'static str,
    pub completed_checks: usize,
}

pub fn scan(
    root: &Path,
    mode: ScanMode,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Progress),
) -> IntegrityReport {
    let mut report = IntegrityReport {
        schema_version: 1,
        build_id: BUILD_ID.into(),
        root: root.to_owned(),
        windows_directory: root.join("Windows"),
        volume: None,
        recovery_location: None,
        windows: None,
        mode,
        started_unix: now(),
        completed_unix: 0,
        status: Status::Blocked,
        checks: vec![],
        limitations: vec![
            "Inspeção passiva e amostral. Estrutura legível não autentica assinatura, catálogo ou conteúdo protegido.".into(),
            "A versão de arquivo corrobora o perfil. Edição, idioma, nível de atualização e recursos instalados continuam indeterminados.".into(),
            "Entradas CBS são históricas. Elas não provam o estado atual de um pacote ou arquivo.".into(),
            "O volume deve estar offline e estável durante a inspeção. A identidade é consultada antes e depois da leitura.".into(),
        ],
        not_executed: vec![
            "Validação completa de catálogos e arquivos protegidos.".into(),
            "Resolução do carregador ativo e dos dispositivos BCD. Consulte Diagnóstico.".into(),
            "Comparação de payload com mídia de referência. Compatibilidade de manutenção não estabelecida.".into(),
            "SFC e DISM offline. Requerem qualificação em clone gravável e destino independente para scratch e logs.".into(),
        ],
        errors: vec![],
    };

    let status = inspect(&mut report, cancel, &mut progress);
    report.status = status.unwrap_or_else(|error| {
        let status = match error.kind {
            ErrorKind::Cancelled => Status::Cancelled,
            ErrorKind::Unsupported => Status::Unsupported,
            _ => Status::Blocked,
        };
        report.errors.push(error);
        status
    });
    report.completed_unix = now();

    report
}

fn inspect(
    report: &mut IntegrityReport,
    cancel: &AtomicBool,
    progress: &mut dyn FnMut(Progress),
) -> Result<Status> {
    check_cancel(cancel)?;
    let root = safe_existing(&report.root)?;
    let volume = identity(&root)?;
    let location = recovery_location(&root)?;
    let windows = safe_existing(&root.join("Windows"))?;

    report.root = root;
    report.windows_directory = windows.clone();
    report.volume = Some(volume);
    report.recovery_location = Some(location.clone());

    if !windows.is_dir() {
        return Err(Error::new(
            ErrorKind::Validation,
            "Windows não é um diretório",
        ));
    }
    if report.mode == ScanMode::NativeVerification {
        return Err(Error::new(
            ErrorKind::Environment,
            "Verificação nativa indisponível neste perfil",
        ));
    }

    notify(progress, report, "Identificando a instalação");
    let Some(profile) = files::profile(&windows)? else {
        report.limitations.push("Não foi possível corroborar a versão 10.0 e a arquitetura x64 pelos binários e hives offline.".into());
        return Ok(Status::Unsupported);
    };
    report.windows = Some(profile);

    notify(progress, report, "Lendo executáveis do sistema");
    files::inspect(report, cancel)?;
    notify(progress, report, "Verificando metadados de manutenção");
    servicing::inspect(report, cancel)?;
    check_cancel(cancel)?;

    match recovery_location(&report.root) {
        Ok(current) if current == location => {}
        result => {
            report.checks.clear();
            return Err(result.err().unwrap_or_else(|| {
                Error::new(
                    ErrorKind::Conflict,
                    "Identidade da origem alterada durante a análise",
                )
            }));
        }
    }

    if report
        .checks
        .iter()
        .any(|check| check.observation.actionable())
    {
        Ok(Status::FindingsDetected)
    } else if report
        .checks
        .iter()
        .any(|check| check.observation == Observation::Inconclusive)
    {
        Ok(Status::Inconclusive)
    } else {
        Ok(Status::NoDetectedIssue)
    }
}

fn notify(progress: &mut dyn FnMut(Progress), report: &IntegrityReport, phase: &'static str) {
    progress(Progress {
        phase,
        completed_checks: report.checks.len(),
    });
}

fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Relaxed) {
        return Err(Error::new(
            ErrorKind::Cancelled,
            "Leitura de integridade interrompida",
        ));
    }

    Ok(())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn missing(error: &Error) -> bool {
    #[cfg(windows)]
    let absent = matches!(error.os_code, Some(2 | 3));
    #[cfg(not(windows))]
    let absent = error.os_code == Some(2);

    error.kind == ErrorKind::Io && absent
}

fn metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    // Only a filesystem absence after the checked path traversal can establish absence.
    match safe_existing(path) {
        Ok(path) => fs::metadata(path)
            .map(Some)
            .map_err(|error| Error::io("Consultar metadados", error)),
        Err(error) if missing(&error) => match fs::symlink_metadata(path) {
            Err(probe) if probe.kind() == io::ErrorKind::NotFound => Ok(None),
            _ => Err(error),
        },
        Err(error) => Err(error),
    }
}

fn unreadable(error: io::Error) -> Error {
    Error::io("Ler componente offline", error)
}

fn record(
    report: &mut IntegrityReport,
    category: Category,
    component: &str,
    path: PathBuf,
    result: (Observation, String, Option<Error>),
) {
    let (observation, evidence, error) = result;
    let next_action = match observation {
        Observation::ConfirmedMissing | Observation::ConfirmedCorruption => {
            "Proteja os dados. Corrobore o achado em clone e confira a configuração de recuperação."
        }
        Observation::SuspectedCorruption | Observation::HistoricalFailure => {
            "Correlacione o pacote e a operação com evidência atual de manutenção em clone."
        }
        Observation::Inconclusive => "Confirme o acesso ao volume e repita a inspeção.",
        _ => "Consulte o escopo da verificação no relatório.",
    };

    report.checks.push(Check {
        identifier: format!(
            "{}.{}",
            match category {
                Category::Boot => "boot",
                Category::System => "system",
                Category::ComponentStore => "store",
                Category::ServicingHistory => "history",
            },
            component.to_ascii_lowercase().replace(' ', "_")
        ),
        category,
        path,
        expected_component: component.into(),
        observation,
        severity: observation.severity(),
        evidence,
        next_action: next_action.into(),
        error,
    });
}

type ResultEvidence = (Observation, String, Option<Error>);

fn evidence(observation: Observation, message: impl Into<String>) -> ResultEvidence {
    (observation, message.into(), None)
}

fn unavailable(error: Error) -> ResultEvidence {
    (
        Observation::Inconclusive,
        error.context.clone(),
        Some(error),
    )
}

#[cfg(test)]
mod tests;
