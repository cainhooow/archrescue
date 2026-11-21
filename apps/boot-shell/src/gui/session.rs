//! Selection routing and continuity across the recovery pages.

use std::{collections::HashMap, path::Path};

use archrescue_backup::portable_relative;
use archrescue_diagnostics::recovery::efi::{self, Resolution};

use super::*;
use crate::{
    paths,
    resources::{Backup, Catalog, Location, Windows},
};

pub const CHOOSE_WINDOWS: usize = 248;
pub const CHOOSE_BACKUP: usize = 249;
pub const SELECT_RECOMMENDED: usize = 250;
pub const DATA_CHECKPOINT: usize = 258;
pub const EFI_CHECKPOINT: usize = 259;

#[derive(Default)]
pub struct Session {
    pub catalog: Catalog,
    pub windows: Option<Windows>,
    pub source: Option<Location>,
    pub efi: Efi,
    pub repair_data: Option<Backup>,
    pub repair_efi: Option<Backup>,
    previous_efi: Option<Location>,
    pub backup: Option<Backup>,
    pub resume: Option<CheckpointEntry>,
    pub backup_verification: Option<Result<()>>,
    pub registered: Vec<PathBuf>,
    destinations: HashMap<usize, Location>,
}

impl Session {
    pub fn destination(&self, section: usize) -> Result<Location> {
        self.destinations
            .get(&section)
            .cloned()
            .ok_or_else(|| Error::new(ErrorKind::Validation, "Escolha uma pasta de destino."))
    }

    pub fn available_windows(&self) -> Option<&Windows> {
        let selected = self.windows.as_ref()?;
        let mut matches = self.catalog.windows.iter().filter(|candidate| {
            candidate.location.same_location(&selected.location)
                && candidate.installation.version == selected.installation.version
                && candidate.installation.architecture == selected.installation.architecture
                && candidate.installation.loader_architecture
                    == selected.installation.loader_architecture
        });
        matches.next().filter(|_| matches.next().is_none())
    }

    pub fn replace_catalog(&mut self, catalog: Catalog) -> bool {
        self.catalog = catalog;
        if self.windows.is_none() && self.catalog.windows.len() == 1 {
            self.windows = self.catalog.windows.first().cloned();
        } else if let Some(candidate) = self.available_windows().cloned() {
            self.windows = Some(candidate);
        }

        self.windows.is_some() && self.available_windows().is_none()
    }

    pub fn accepts_efi(&self, generation: u64, source: &Location, refresh_pending: bool) -> bool {
        !refresh_pending
            && generation == self.catalog.generation
            && self
                .available_windows()
                .is_some_and(|windows| windows.location.same_location(source))
    }

    pub fn invalidate_repair(&mut self) {
        if let Some(location) = self.efi.location() {
            self.previous_efi = Some(location.clone());
        }
        self.efi = Efi::Detecting;
        self.repair_data = None;
        self.repair_efi = None;
    }

    pub fn accept_efi(&mut self, mut efi: Efi) {
        if let Some(previous) = self.previous_efi.take() {
            let changed = efi
                .location()
                .is_none_or(|location| !previous.same_location(location));
            if changed {
                efi = Efi::Changed(Box::new(efi));
            }
        }

        self.efi = efi;
    }

    pub fn revalidate_selections(&mut self) -> bool {
        let before = self.destinations.len();
        self.destinations
            .retain(|_, location| location.revalidate().is_ok());

        let source_changed = self
            .source
            .as_ref()
            .is_some_and(|source| source.revalidate().is_err());
        if source_changed {
            self.source = None;
        }
        let backup_changed = self
            .backup
            .as_ref()
            .is_some_and(|backup| backup.revalidate().is_err());
        if backup_changed {
            self.backup = None;
        }
        source_changed || backup_changed || before != self.destinations.len()
    }

    pub fn sources(&self, section: usize) -> Vec<Location> {
        let windows = self
            .catalog
            .windows
            .iter()
            .map(|windows| windows.location.clone());
        match section {
            BACKUP => self.source.iter().cloned().collect(),
            REPAIR_PAGE => self
                .windows
                .iter()
                .map(|windows| windows.location.clone())
                .chain(self.efi.location().cloned())
                .collect(),
            _ => windows.chain(self.source.clone()).collect(),
        }
    }
}

#[derive(Default)]
pub enum Efi {
    #[default]
    Pending,
    Detecting,
    Identified {
        location: Location,
        partition: u32,
        disk: u32,
    },
    Blocked(Resolution),
    Changed(Box<Efi>),
}

impl Efi {
    pub fn location(&self) -> Option<&Location> {
        match self {
            Self::Identified { location, .. } => Some(location),
            Self::Changed(current) => current.location(),
            _ => None,
        }
    }

    pub fn details(&self, technician: bool) -> String {
        match self {
            Self::Pending => "Escolha uma instalação Windows disponível para identificar a partição de inicialização.".into(),
            Self::Detecting => "Identificando a ESP no disco do Windows selecionado…".into(),
            Self::Identified { location, partition, disk } => format!(
                "Alvo identificado · Disco {disk} · Partição {partition}
{}
Associação pelo mesmo disco GPT. A referência do BCD ainda precisa ser comprovada.

Preserve os dados selecionados e a árvore EFI em outro disco. Escolha uma pasta gravável para gerar a prévia.",
                if technician { location.label(true) } else { paths::friendly(&location.path) },
            ),
            Self::Blocked(Resolution::Missing) => "Nenhuma ESP no disco deste Windows. Confira a conexão e escolha outra instalação, se disponível. Atualize o diagnóstico depois de conferir o disco.".into(),
            Self::Blocked(Resolution::Ambiguous) => "Mais de uma ESP ou identidade EFI divergente. O reparo permanece bloqueado porque ainda não há um alvo comprovado.\n\nUse Consultar partições EFI para revisar as candidatas ou escolher outro Windows encontrado. Depois de conferir a conexão do disco, use Atualizar. Não é necessário digitar o caminho da ESP.".into(),
            Self::Blocked(Resolution::Unsupported(reason) | Resolution::Inaccessible(reason)) => format!(
                "ESP inacessível ou incompatível. {reason}
Confira o disco e as permissões, depois atualize o diagnóstico."
            ),
            Self::Changed(current) => format!(
                "O alvo mudou. A prévia anterior foi descartada. Confira o resultado atual antes de continuar.

{}",
                current.details(technician)
            ),
            Self::Blocked(Resolution::Identified { .. }) => unreachable!("identified targets carry a captured location"),
        }
    }

    pub fn discover(windows: &Windows) -> Self {
        let result = (|| {
            let root = windows.revalidate()?;
            let inventory = archrescue_storage::inventory();
            let resolved = efi::discover(&root, &inventory);
            let Resolution::Identified {
                root,
                volume,
                partition,
            } = resolved
            else {
                return Ok(Self::Blocked(resolved));
            };
            let location = Location::capture(&root)?;
            let mounted = archrescue_storage::windows::mounted_partition(&location.path)?;
            let source = archrescue_storage::windows::mounted_partition(&windows.location.path)?;
            if !mounted.volume_guid.eq_ignore_ascii_case(&volume)
                || mounted.disk_number != source.disk_number
                || mounted.disk_id != source.disk_id
                || mounted.partition.id != partition.id
                || mounted.partition.offset != partition.offset
                || mounted.partition.length != partition.length
                || !mounted.partition.efi
                || mounted.filesystem != "FAT32"
            {
                return Err(Error::new(
                    ErrorKind::Conflict,
                    "A identidade da ESP mudou durante a leitura.",
                ));
            }

            Ok(Self::Identified {
                location,
                partition: partition.number,
                disk: mounted.disk_number,
            })
        })();

        result.unwrap_or_else(|error: Error| {
            Self::Blocked(Resolution::Inaccessible(error.to_string()))
        })
    }
}

pub unsafe fn discover_efi(hwnd: HWND, app: &mut App) {
    app.current_repair = None;
    let Some(windows) = app.session.available_windows().cloned() else {
        app.session.efi = Efi::Pending;
        return;
    };
    app.session.invalidate_repair();
    let generation = app.session.catalog.generation;
    set(hwnd, STATUS, "Identificando a partição de inicialização…");
    work(app, move || {
        let efi = Efi::discover(&windows);
        Event::Efi(generation, windows.location, efi)
    });
}

pub fn request(app: &mut App, purpose: picker::Purpose) {
    app.picker = Some(purpose);
}

pub unsafe fn run_picker(hwnd: HWND, state: &WindowState, purpose: picker::Purpose) {
    let request = {
        let app = state.app.borrow();
        let Some(report) = &app.report else {
            set(hwnd, STATUS, "Aguarde o inventário de recursos.");
            return;
        };
        let minimum = match purpose {
            picker::Purpose::SaveFiles => app.preview.as_ref().map_or(0, |preview| preview.bytes),
            picker::Purpose::Preserve => 64 * 1024 * 1024,
            _ => 0,
        };

        let mut sources = app.session.sources(app.section);
        if matches!(
            purpose,
            picker::Purpose::DeploymentDisk | picker::Purpose::Handoff
        ) {
            sources.extend(
                app.session
                    .backup
                    .iter()
                    .map(|backup| backup.location.clone()),
            );
        }
        if purpose == picker::Purpose::DeploymentDisk {
            if let Some(system) = std::env::var_os("SystemRoot") {
                if let Ok(location) = Location::capture(Path::new(&system)) {
                    sources.push(location);
                }
            }
        }
        picker::Request::new(
            purpose,
            report,
            &app.session.catalog,
            app.session.available_windows(),
            (sources, minimum),
            app.technician,
            app.session
                .resume
                .as_ref()
                .filter(|_| purpose == picker::Purpose::ResumeSource)
                .and_then(|entry| entry.manifest.as_ref())
                .map(|manifest| manifest.source_identity.clone()),
        )
    };
    let Some(selection) = picker::show(hwnd, state, request) else {
        return;
    };

    let mut app = state.app.borrow_mut();
    if app.busy || app.refresh_pending {
        return;
    }
    match selection {
        picker::Selection::Windows(windows) => select_windows(hwnd, &mut app, *windows),
        picker::Selection::Location(location) => match purpose {
            picker::Purpose::Source => browse(hwnd, &mut app, location),
            picker::Purpose::ResumeSource => {
                if let Some(entry) = app.session.resume.take() {
                    confirm_resume(&mut app, entry, location);
                }
            }
            picker::Purpose::Media => deployment::inspect(hwnd, &mut app, location),
            _ => {
                if purpose == picker::Purpose::SaveFiles {
                    app.preview = None;
                    app.checkpoints = None;
                }
                if purpose == picker::Purpose::Preserve {
                    app.current_repair = None;
                    app.session.repair_data = None;
                    app.session.repair_efi = None;
                }
                let section = app.section;
                app.session.destinations.insert(section, location);
            }
        },
        picker::Selection::Disk(disk) => deployment::select_disk(&mut app, disk),
        picker::Selection::Backup(backup) if purpose.repair_checkpoint() => {
            verify_repair_checkpoint(hwnd, &mut app, purpose, backup);
        }
        picker::Selection::Backup(backup) if purpose == picker::Purpose::Resume => {
            let entry = transaction::checkpoint(&backup.location.path);
            resolve_resume(hwnd, &mut app, entry);
        }
        picker::Selection::Backup(backup) => {
            app.session.backup = Some(backup);
            app.session.backup_verification = None;
            deployment::invalidate_plan(&mut app);
        }
    }
    render(hwnd, &app);
    let confirmation = app.confirmation.take();
    let next_picker = app.picker.take();
    drop(app);
    if let Some(confirmation) = confirmation {
        confirmation.ask(hwnd, state);
    } else if let Some(purpose) = next_picker {
        run_picker(hwnd, state, purpose);
    }
}

unsafe fn verify_repair_checkpoint(
    hwnd: HWND,
    app: &mut App,
    purpose: picker::Purpose,
    backup: Backup,
) {
    let Ok(destination) = app.session.destination(REPAIR_PAGE) else {
        return;
    };
    let source = if purpose == picker::Purpose::DataCheckpoint {
        app.session
            .available_windows()
            .map(|windows| windows.location.clone())
    } else {
        app.session.efi.location().cloned()
    };
    let Some(source) = source else {
        return;
    };

    let cancel = Arc::new(AtomicBool::new(false));
    app.cancel = cancel.clone();
    work(app, move || {
        let result = (|| {
            let checkpoint = backup.revalidate()?;
            let destination = destination.revalidate()?;
            if checkpoint.parent() != Some(&destination) {
                return Err(Error::new(
                    ErrorKind::Validation,
                    "Escolha um checkpoint na pasta de preservação selecionada.",
                ));
            }
            let root = source.revalidate()?;
            let verified = transaction::verify(&root, &checkpoint, &cancel)?;
            if verified.manifest.files.is_empty()
                || (purpose == picker::Purpose::EfiCheckpoint
                    && verified.manifest.selections != [PathBuf::from("EFI")])
            {
                return Err(Error::new(ErrorKind::Validation,
                    "O checkpoint precisa conter arquivos. A preservação da ESP exige a árvore EFI inteira."));
            }
            Ok(Event::RepairCheckpoint(purpose, backup))
        })();
        result.unwrap_or_else(Event::Failure)
    });
    app.cancellable = true;
    set(
        hwnd,
        STATUS,
        "Conferindo o checkpoint, a origem e os arquivos publicados…",
    );
}

pub unsafe fn select_windows(hwnd: HWND, app: &mut App, windows: Windows) {
    let changed = app
        .session
        .available_windows()
        .is_none_or(|previous| !previous.location.same_location(&windows.location));
    pages::select_root(hwnd, app, &windows.installation.volume);
    app.session.windows = Some(windows.clone());
    if changed {
        app.current_repair = None;
        app.session.efi = Efi::Pending;
        app.session.repair_data = None;
        app.session.repair_efi = None;
        app.session.previous_efi = None;
        app.session.source = None;
        app.entries.clear();
        app.listing_loaded = false;
        app.preview = None;
        app.checkpoints = None;
    }
    if app.section == REPAIR_PAGE {
        discover_efi(hwnd, app);
    }

    if app.section == BACKUP {
        recommended(hwnd, app, windows);
    }
}

pub unsafe fn recommended(hwnd: HWND, app: &mut App, windows: Windows) {
    app.preview = None;
    app.checkpoints = None;
    app.session.source = None;
    app.listing_loaded = false;
    app.recommended = false;
    set(hwnd, STATUS, "Procurando pastas de perfis neste Windows…");
    work(app, move || match recommended_folders(&windows) {
        Ok(entries) => Event::Listing(windows.location, entries, true),
        Err(error) => Event::Failure(error),
    });
    render(hwnd, app);
}

fn recommended_folders(windows: &Windows) -> Result<Vec<Entry>> {
    let root = windows.revalidate()?;
    let users = root.join("Users");
    let profiles = archrescue_backup::browse(&users)?;
    let mut entries = Vec::new();

    // Offline Known Folder redirection is not inferred. Only readable, local
    // conventional folders are suggested. Custom browsing covers other locations.
    for profile in profiles
        .into_iter()
        .filter(|entry| entry.directory && entry.error.is_none())
    {
        if ["default", "default user", "public", "all users"]
            .iter()
            .any(|name| profile.name.to_string_lossy().eq_ignore_ascii_case(name))
        {
            continue;
        }
        let relative = Path::new("Users").join(profile.name);
        let home = root.join(&relative);
        if !archrescue_storage::safe_existing(&home.join("NTUSER.DAT"))
            .is_ok_and(|path| path.is_file())
        {
            continue;
        }
        for name in [
            "Desktop",
            "Documents",
            "Downloads",
            "Pictures",
            "Videos",
            "Music",
        ] {
            let name = relative.join(name);
            if portable_relative(&name).is_err() {
                continue;
            }
            let path = root.join(&name);
            if let Ok(path) = archrescue_storage::safe_existing(&path) {
                if path.is_dir() && std::fs::read_dir(path).is_ok() {
                    entries.push(Entry {
                        name,
                        directory: true,
                        bytes: 0,
                        modified_unix: None,
                        error: None,
                    });
                }
            }
        }
    }

    Ok(entries)
}

pub fn folder_label(path: &Path) -> String {
    let folder = match path.file_name().and_then(|name| name.to_str()) {
        Some("Desktop") => "Área de Trabalho",
        Some("Documents") => "Documentos",
        Some("Pictures") => "Imagens",
        Some("Videos") => "Vídeos",
        Some("Music") => "Músicas",
        Some("Downloads") => "Downloads",
        _ => return paths::friendly(path),
    };
    let profile = path
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_default()
        .to_string_lossy();
    format!("{folder} · {profile}")
}

pub unsafe fn render_fields(hwnd: HWND, app: &App) {
    let purpose = match app.section {
        BACKUP => picker::Purpose::SaveFiles,
        REPAIR_PAGE => picker::Purpose::Preserve,
        REPORTS => picker::Purpose::Report,
        deployment::DEPLOY => picker::Purpose::Plan,
        deployment::RESTORE => picker::Purpose::Handoff,
        console::PAGE => picker::Purpose::ConsoleLog,
        _ => picker::Purpose::Report,
    };
    let label = app.session.destination(app.section).map_or_else(
        |_| purpose.title().into(),
        |location| location.label(app.technician),
    );
    set(hwnd, DEST, &label);
    let source = app.session.source.as_ref().map_or_else(
        || "Escolher pasta de origem".into(),
        |location| location.label(app.technician),
    );
    set(hwnd, ADDRESS, &source);
    let windows = app.session.available_windows().map_or_else(
        || "Escolher Windows".into(),
        |windows| windows.label(app.technician),
    );
    set(hwnd, CHOOSE_WINDOWS, &windows);
    let efi = app.session.efi.location().map_or_else(
        || match app.session.efi {
            Efi::Detecting => "Identificando a ESP…".into(),
            _ => "Consultar partições EFI".into(),
        },
        |location| location.label(app.technician),
    );
    set(hwnd, ESP, &efi);
    for (id, checkpoint, prompt) in [
        (
            DATA_CHECKPOINT,
            &app.session.repair_data,
            "Escolher backup de dados",
        ),
        (
            EFI_CHECKPOINT,
            &app.session.repair_efi,
            "Escolher backup EFI",
        ),
    ] {
        let label = checkpoint.as_ref().map_or_else(
            || prompt.into(),
            |backup| {
                if app.technician {
                    backup.location.label(true)
                } else if id == DATA_CHECKPOINT {
                    "Backup de dados verificado".into()
                } else {
                    "Backup EFI verificado".into()
                }
            },
        );
        set(hwnd, id, &label);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows(path: &str, id: &str) -> Windows {
        Windows {
            installation: archrescue_windows_detection::Installation {
                path: Path::new(path).join("Windows"),
                volume: path.into(),
                architecture: Some("x64".into()),
                version: Some("10.0.19045".into()),
                loader_architecture: Some("x64".into()),
                recovery_files: Vec::new(),
                confidence: String::new(),
                evidence: Vec::new(),
                limitations: Vec::new(),
            },
            location: Location::test_location(path, id),
        }
    }

    #[test]
    fn same_volume_remap_preserves_windows_and_rejects_delayed_efi() {
        let original = windows(r"C:\", "source");
        let remapped = windows(r"W:\", "source");
        let mut session = Session::default();
        assert!(!session.replace_catalog(Catalog {
            generation: 1,
            windows: vec![original.clone()],
            ..Default::default()
        }));
        assert!(session.accepts_efi(1, &original.location, false));
        assert!(!session.accepts_efi(1, &original.location, true));

        assert!(!session.replace_catalog(Catalog {
            generation: 2,
            windows: vec![remapped.clone()],
            ..Default::default()
        }));
        assert_eq!(
            session.available_windows().unwrap().location.path,
            remapped.location.path
        );
        assert!(!session.accepts_efi(1, &original.location, false));

        assert!(session.replace_catalog(Catalog {
            generation: 3,
            windows: vec![windows(r"W:\", "replacement")],
            ..Default::default()
        }));
        assert!(!session.accepts_efi(3, &original.location, false));
        assert!(session.available_windows().is_none());
    }

    #[test]
    fn changed_efi_retains_the_current_blocker_and_revokes_the_location() {
        let mut session = Session {
            efi: Efi::Identified {
                location: Location::test_location(r"S:\", "esp"),
                partition: 1,
                disk: 0,
            },
            ..Default::default()
        };

        session.invalidate_repair();
        session.accept_efi(Efi::Blocked(Resolution::Ambiguous));

        assert!(session.efi.location().is_none());
        let text = session.efi.details(false);
        assert!(text.contains("O alvo mudou"));
        assert!(text.contains("Mais de uma ESP"));
    }
}
