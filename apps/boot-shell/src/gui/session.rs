//! Selection routing and continuity across the recovery pages.

use std::{collections::HashMap, path::Path};

use archrescue_backup::portable_relative;

use super::*;
use crate::{
    paths,
    resources::{Backup, Catalog, Location, Windows},
};

pub const CHOOSE_WINDOWS: usize = 248;
pub const CHOOSE_BACKUP: usize = 249;
pub const SELECT_RECOMMENDED: usize = 250;

#[derive(Default)]
pub struct Session {
    pub catalog: Catalog,
    pub windows: Option<Windows>,
    pub suggested_efi: Option<Location>,
    pub source: Option<Location>,
    pub efi: Option<Location>,
    pub backup: Option<Backup>,
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
        self.catalog.windows.iter().find(|candidate| {
            candidate.location.same_location(&selected.location)
                && candidate.installation.version == selected.installation.version
                && candidate.installation.architecture == selected.installation.architecture
                && candidate.installation.loader_architecture
                    == selected.installation.loader_architecture
        })
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

    pub fn suggest_efi(&mut self, report: &Report) {
        self.suggested_efi = self.available_windows().and_then(|windows| {
            let association = report
                .diagnosis
                .associations
                .iter()
                .find(|association| association.installation == windows.installation.path)?;
            let [path] = association.mounted_efi.as_slice() else {
                return None;
            };
            if association.efi_partitions.len() != 1 {
                return None;
            }

            self.catalog
                .efi
                .iter()
                .find(|(candidate, _)| candidate == path)
                .map(|(_, location)| location.clone())
        });
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
                .chain(self.efi.clone().or_else(|| self.suggested_efi.clone()))
                .collect(),
            _ => windows.chain(self.source.clone()).collect(),
        }
    }
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

        picker::Request::new(
            purpose,
            report,
            &app.session.catalog,
            app.session.available_windows(),
            app.session.sources(app.section),
            minimum,
            app.technician,
        )
    };
    let Some(selection) = picker::show(hwnd, state, request) else {
        return;
    };

    let mut app = state.app.borrow_mut();
    if app.busy {
        return;
    }
    match selection {
        picker::Selection::Windows(windows) => select_windows(hwnd, &mut app, *windows),
        picker::Selection::Location(location) => match purpose {
            picker::Purpose::Source => browse(hwnd, &mut app, location),
            picker::Purpose::Efi => {
                app.session.efi = Some(location);
                app.current_repair = None;
            }
            picker::Purpose::Media => deployment::inspect(hwnd, &mut app, location),
            _ => {
                if purpose == picker::Purpose::SaveFiles {
                    app.preview = None;
                    app.checkpoints = None;
                }
                if purpose == picker::Purpose::Preserve {
                    app.current_repair = None;
                }
                let section = app.section;
                app.session.destinations.insert(section, location);
            }
        },
        picker::Selection::Disk(disk) => deployment::select_disk(&mut app, disk),
        picker::Selection::Backup(backup) => {
            app.session.backup = Some(backup);
            app.session.backup_verification = None;
            deployment::invalidate_plan(&mut app);
        }
    }
    render(hwnd, &app);
}

pub unsafe fn select_windows(hwnd: HWND, app: &mut App, windows: Windows) {
    let changed = app
        .session
        .windows
        .as_ref()
        .is_none_or(|previous| !previous.location.same_location(&windows.location));
    if changed {
        app.session.efi = None;
        app.current_repair = None;
    }
    pages::select_root(hwnd, app, &windows.installation.volume);
    app.session.windows = Some(windows.clone());
    if let Some(report) = &app.report {
        app.session.suggest_efi(report);
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
    let efi = app
        .session
        .efi
        .as_ref()
        .or(app.session.suggested_efi.as_ref())
        .map_or_else(
            || "Identificação pendente · consultar ESP".into(),
            |location| location.label(app.technician),
        );
    set(hwnd, ESP, &efi);
}
