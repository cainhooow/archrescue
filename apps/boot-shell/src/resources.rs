//! Recovery session inventory and validated selections. Display strings never become paths.

use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use archrescue_backup::transaction::{self, CheckpointEntry};
use archrescue_core::{Error, ErrorKind, Result};
use archrescue_evidence::Report;
use archrescue_storage::{self as storage, Inventory, RecoveryLocation, Space, Volume};
use archrescue_windows_detection::Installation;

use crate::paths;

const BACKUP_PREFIX: &str = "archrescue-backup-";
const ENTRY_LIMIT: usize = 1024;
const BACKUP_LIMIT: usize = 64;
const INDEX_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Identity {
    Partition(RecoveryLocation),
    Optical { volume: String, serial: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Location {
    pub path: PathBuf,
    identity: Identity,
}

impl Location {
    pub fn capture(path: &Path) -> Result<Self> {
        let path = storage::safe_existing(path)?;
        let identity = match storage::recovery_location(&path) {
            Ok(identity) => Identity::Partition(identity),
            Err(error) => optical_identity(&path).ok_or(error)?,
        };

        Ok(Self {
            path,
            identity,
        })
    }

    pub fn revalidate(&self) -> Result<PathBuf> {
        let current = Self::capture(&self.path)?;
        if current.identity != self.identity {
            return Err(Error::new(
                ErrorKind::Conflict,
                "O volume ou caminho selecionado mudou. Escolha o recurso novamente.",
            ));
        }

        Ok(current.path)
    }

    pub fn same_location(&self, other: &Self) -> bool {
        self.identity == other.identity
    }

    pub fn same_partition(&self, other: &Self) -> bool {
        match (&self.identity, &other.identity) {
            (Identity::Partition(left), Identity::Partition(right)) => {
                left.filesystem_id == right.filesystem_id
            }
            (Identity::Optical { .. }, Identity::Optical { .. }) => self.identity == other.identity,
            _ => false,
        }
    }

    pub fn recorded_location(&self) -> Option<&RecoveryLocation> {
        match &self.identity {
            Identity::Partition(identity) => Some(identity),
            Identity::Optical { .. } => None,
        }
    }

    pub fn label(&self, technician: bool) -> String {
        paths::display(&self.path, technician)
    }
}

fn optical_identity(path: &Path) -> Option<Identity> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetVolumeInformationW};
    use windows_sys::Win32::System::WindowsProgramming::DRIVE_CDROM;

    let mut root = path
        .ancestors()
        .find(|path| path.parent().is_none())?
        .to_path_buf();
    if !root.has_root() {
        return None;
    }
    // A prefixed Windows path may have a prefix-only ancestor after the root.
    if let Some(component) = path.components().nth(1) {
        root.push(component.as_os_str());
    }
    let volume = storage::filesystem_name(path).ok()?;
    let root: Vec<_> = root.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut serial = 0;

    // SAFETY: root is NUL-terminated and serial is writable for the call.
    unsafe {
        if GetDriveTypeW(root.as_ptr()) != DRIVE_CDROM
            || GetVolumeInformationW(
                root.as_ptr(),
                std::ptr::null_mut(),
                0,
                &mut serial,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            ) == 0
        {
            return None;
        }
    }

    Some(Identity::Optical {
        volume,
        serial,
    })
}

#[derive(Clone)]
pub struct Windows {
    pub installation: Installation,
    pub location: Location,
}

impl Windows {
    pub fn revalidate(&self) -> Result<PathBuf> {
        let path = self.location.revalidate()?;
        let discovery = archrescue_windows_detection::discover(std::slice::from_ref(&path));
        if !discovery.installations.iter().any(|item| {
            item.architecture == self.installation.architecture
                && item.loader_architecture == self.installation.loader_architecture
                && item.version == self.installation.version
        }) {
            return Err(Error::new(
                ErrorKind::Conflict,
                "As evidências da instalação selecionada mudaram. Atualize o diagnóstico.",
            ));
        }

        Ok(path)
    }

    pub fn label(&self, technician: bool) -> String {
        let build = self.installation.version.as_deref().unwrap_or("offline");
        format!("Windows {build} · {}", self.location.label(technician))
    }
}

#[derive(Clone)]
pub struct Backup {
    pub location: Location,
    pub operation: String,
    pub fingerprint: String,
}

impl Backup {
    pub fn revalidate(&self) -> Result<PathBuf> {
        let path = self.location.revalidate()?;
        let manifest = transaction::manifest(&path)?;
        if manifest.operation_id != self.operation || manifest.plan_fingerprint != self.fingerprint
        {
            return Err(Error::new(
                ErrorKind::Integrity,
                "A operação de backup selecionada mudou.",
            ));
        }

        Ok(path)
    }
}

#[derive(Default)]
pub struct Catalog {
    pub generation: u64,
    pub windows: Vec<Windows>,
    pub backups: Vec<(CheckpointEntry, Location)>,
    pub media: Vec<Location>,
    pub volumes: Vec<(String, Location)>,
    pub efi: Vec<(PathBuf, Location)>,
    pub warnings: Vec<String>,
}

impl Catalog {
    pub fn discover(report: &Report, registered: &[PathBuf], generation: u64) -> Self {
        let mut catalog = Self {
            generation,
            ..Self::default()
        };
        for installation in &report.installations.installations {
            match Location::capture(&installation.volume) {
                Ok(location) => catalog.windows.push(Windows {
                    installation: installation.clone(),
                    location,
                }),
                Err(error) => catalog.warnings.push(error.context),
            }
        }

        for association in &report.diagnosis.associations {
            for path in &association.mounted_efi {
                if let Ok(location) = Location::capture(path) {
                    catalog.efi.push((path.clone(), location));
                }
            }
        }

        let mut parents = registered.to_vec();
        let mut seen = HashSet::new();
        for volume in &report.inventory.volumes {
            let Some(root) = volume_root(volume) else {
                continue;
            };

            if let Ok(location) = Location::capture(&root) {
                catalog.volumes.push((volume.identity.clone(), location));
            }
            for image in [
                "sources/install.wim",
                "sources/install.esd",
                "install.wim",
                "install.esd",
            ] {
                let candidate = root.join(image);
                if let Ok(location) = Location::capture(&candidate) {
                    if location.path.is_file() && seen.insert(location.path.clone()) {
                        catalog.media.push(location);
                    }
                }
            }

            parents.extend([
                root.join("ArchRescue").join("Backups"),
                root.join("ArchRescue"),
                root,
            ]);
        }
        catalog.index_backups(parents);
        catalog
    }

    fn index_backups(&mut self, parents: Vec<PathBuf>) {
        let mut parents_seen = HashSet::new();
        let mut fingerprints = HashSet::new();
        let mut bytes_left = INDEX_BYTES;

        for parent in parents {
            let Ok(parent) = storage::safe_existing(&parent) else {
                continue;
            };
            if !parents_seen.insert(parent.clone()) {
                continue;
            }
            let Ok(children) = fs::read_dir(&parent) else {
                continue;
            };

            for child in children.take(ENTRY_LIMIT).flatten() {
                if !child
                    .file_name()
                    .to_string_lossy()
                    .starts_with(BACKUP_PREFIX)
                {
                    continue;
                }
                if self.backups.len() >= BACKUP_LIMIT {
                    self.warnings
                        .push("Limite de descoberta de backups atingido.".into());
                    return;
                }

                let path = child.path();
                let location = Location::capture(&path).or_else(|_| Location::capture(&parent));
                let Ok(location) = location else {
                    continue;
                };
                let records = ["manifest.json", "status.json"];
                let bytes = records
                    .iter()
                    .filter_map(|name| storage::safe_existing(&path.join(name)).ok())
                    .filter_map(|record| fs::metadata(record).ok())
                    .fold(0u64, |bytes, metadata| bytes.saturating_add(metadata.len()));

                let entry =
                    if let Some(remaining) = bytes_left.checked_sub(bytes) {
                        bytes_left = remaining;
                        transaction::checkpoint(&path)
                    } else {
                        CheckpointEntry {
                        directory: path,
                        manifest: None,
                        status: None,
                        error: Some(Error::new(ErrorKind::Unsupported,
                            "Limite de leitura automática. Abra a pasta do backup para validar.")),
                    }
                    };
                if entry.manifest.as_ref().is_none_or(|manifest| {
                    fingerprints.insert((
                        manifest.operation_id.clone(),
                        manifest.plan_fingerprint.clone(),
                    ))
                }) {
                    self.backups.push((entry, location));
                }
            }
        }

        self.backups.sort_by_key(|(entry, _)| {
            std::cmp::Reverse(
                entry
                    .manifest
                    .as_ref()
                    .map(|manifest| manifest.created_unix),
            )
        });
    }
}

pub fn volume_root(volume: &Volume) -> Option<PathBuf> {
    volume.paths.first().cloned().or_else(|| {
        volume
            .filesystem
            .as_ref()
            .map(|_| PathBuf::from(&volume.identity))
    })
}

pub fn writable(destination: &Path, sources: &[Location], minimum_bytes: u64) -> Result<Space> {
    let destination = storage::safe_existing(destination)?;
    let partition = storage::windows::mounted_partition(&destination)?;
    let destination_id = storage::identity(&destination)?;

    if let Some(system) = std::env::var_os("SystemRoot") {
        let system_volume = storage::filesystem_name(Path::new(&system))?;
        if system_volume == destination_id.volume {
            return Err(Error::new(
                ErrorKind::Permission,
                "Volume do ambiente em execução.",
            ));
        }
        if let Ok(system_id) = storage::identity(Path::new(&system)) {
            storage::require_separate(&system_id, &destination_id)?;
        }
    }

    for source in sources {
        let source = source.revalidate()?;
        storage::require_separate(&storage::identity(&source)?, &destination_id)?;
    }

    if partition.partition.efi || partition.filesystem.is_empty() {
        return Err(Error::new(
            ErrorKind::Permission,
            "Partição EFI ou sistema de arquivos indisponível.",
        ));
    }
    storage::windows::require_writable_disk(&format!(
        r"\\.\PhysicalDrive{}",
        partition.disk_number
    ))?;
    let space = storage::space(&destination)?;
    if space.available < minimum_bytes {
        return Err(Error::new(
            ErrorKind::Validation,
            "Espaço insuficiente no destino.",
        ));
    }

    Ok(space)
}

pub fn volume_description(volume: &Volume, inventory: &Inventory, technician: bool) -> String {
    let access = volume.paths.first().map_or_else(
        || "Volume sem letra".into(),
        |path| paths::display(path, technician),
    );
    let disks = volume
        .physical_disks
        .iter()
        .filter(|number| **number != u32::MAX)
        .map(|number| {
            let disk = inventory
                .disks
                .iter()
                .find(|disk| disk.number == Some(*number));
            let kind =
                if disk.is_some_and(|disk| disk.removable == Some(true) || disk.bus == Some(7)) {
                    "Disco externo"
                } else {
                    "Disco"
                };
            let protection = if disk.is_some_and(|disk| disk.read_only == Some(true)) {
                " · Somente leitura"
            } else {
                ""
            };
            format!("{kind} {number}{protection}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let space = volume.space.as_ref().map_or_else(
        || "Espaço indisponível".into(),
        |space| format!("{} livres de {}", size(space.available), size(space.total)),
    );

    let device = if disks.is_empty() {
        String::new()
    } else {
        format!(" · {disks}")
    };
    format!(
        "{access}{device} · {} · {space}",
        volume
            .filesystem
            .as_deref()
            .unwrap_or("Sistema de arquivos indisponível")
    )
}

pub fn size(bytes: u64) -> String {
    if bytes < 1024 * 1024 {
        format!("{bytes} bytes")
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GiB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
