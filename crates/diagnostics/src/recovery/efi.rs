//! Read-only correlation of an offline Windows partition and its GPT ESP.

use std::path::PathBuf;

#[cfg(any(windows, test))]
use archrescue_core::{Error, ErrorKind, Result};
use archrescue_storage::Partition;
#[cfg(any(windows, test))]
use archrescue_storage::{Disk, Inventory};

#[cfg(windows)]
use std::path::Path;

#[cfg(windows)]
use archrescue_storage::windows::{is_partition_guid, mounted_partition, MountedPartition};

#[cfg(windows)]
const ESP_TYPE: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";

#[derive(Debug, Clone)]
pub enum Resolution {
    Missing,
    Ambiguous,
    Unsupported(String),
    Inaccessible(String),
    Identified {
        root: PathBuf,
        volume: String,
        partition: Partition,
    },
}

#[cfg(windows)]
pub(super) fn resolve(source: &MountedPartition, inventory: &Inventory) -> Resolution {
    if source.style != "GPT"
        || source.filesystem != "NTFS"
        || source.partition.efi
        || !source
            .partition
            .id
            .as_deref()
            .is_some_and(is_partition_guid)
    {
        return Resolution::Unsupported(
            "O Windows precisa estar em uma partição NTFS identificada de um disco GPT.".into(),
        );
    }

    let disk = match source_disk(inventory, &source.disk_id, source.disk_number) {
        Ok(disk) => disk,
        Err(error) => return Resolution::Unsupported(error.context),
    };
    if !disk
        .partitions
        .iter()
        .any(|partition| same_partition(partition, &source.partition))
        || !unique_partition(inventory, &source.partition)
    {
        return Resolution::Unsupported(
            "A identidade da partição Windows está incompleta ou duplicada.".into(),
        );
    }

    let candidates = disk
        .partitions
        .iter()
        .filter(|partition| partition.kind == ESP_TYPE)
        .collect::<Vec<_>>();
    let partition = match candidates.as_slice() {
        [] => return Resolution::Missing,
        [partition] => *partition,
        _ => return Resolution::Ambiguous,
    };
    if !partition.id.as_deref().is_some_and(is_partition_guid)
        || !unique_partition(inventory, partition)
    {
        return Resolution::Unsupported(
            "O identificador GPT da ESP está ausente ou duplicado.".into(),
        );
    }

    let mut identified: Option<(PathBuf, String, u32)> = None;
    let mut reason = "A ESP no mesmo disco não tem um volume acessível.".into();
    for volume in &inventory.volumes {
        if volume.physical_disks != [source.disk_number] {
            continue;
        }
        let roots = volume
            .paths
            .iter()
            .map(PathBuf::as_path)
            .chain([Path::new(&volume.identity)]);
        for root in roots {
            let Ok(target) = mounted_partition(root) else {
                continue;
            };
            if target.disk_number != source.disk_number
                || target.disk_id != source.disk_id
                || !same_partition(&target.partition, partition)
            {
                continue;
            }
            if target.filesystem != "FAT32" {
                return Resolution::Unsupported("A ESP identificada precisa usar FAT32.".into());
            }
            if !target.volume_guid.eq_ignore_ascii_case(&volume.identity) {
                reason = "O acesso ao volume EFI mudou. Atualize o diagnóstico.".into();
                continue;
            }
            let root = target.mount_path;
            if std::fs::read_dir(&root).is_err() {
                reason = "O volume EFI não pode ser lido. Confira conexão e permissões.".into();
                continue;
            }
            if let Some((_, _, serial)) = &identified {
                if *serial != target.serial {
                    return Resolution::Ambiguous;
                }
            } else {
                identified = Some((root, target.volume_guid, target.serial));
            }
            break;
        }
    }

    match identified {
        Some((root, volume, _)) => Resolution::Identified {
            root,
            volume,
            partition: partition.clone(),
        },
        None => Resolution::Inaccessible(reason),
    }
}

#[cfg(any(windows, test))]
fn source_disk<'a>(inventory: &'a Inventory, disk_id: &str, number: u32) -> Result<&'a Disk> {
    let mut matches = inventory
        .disks
        .iter()
        .filter(|disk| disk.identity == disk_id);
    let disk = matches.next().filter(|disk| {
        matches.next().is_none()
            && disk.number == Some(number)
            && disk.style.as_deref() == Some("GPT")
    });

    disk.filter(|_| {
        inventory.errors.is_empty()
            && inventory
                .disks
                .iter()
                .all(|disk| disk.style.is_some() && disk.number.is_some())
    })
    .ok_or_else(|| {
        Error::new(
            ErrorKind::Unsupported,
            "O inventário GPT está incompleto ou a identidade do disco é ambígua.",
        )
    })
}

#[cfg(windows)]
fn same_partition(left: &Partition, right: &Partition) -> bool {
    left.id == right.id
        && left.offset == right.offset
        && left.length == right.length
        && left.kind == right.kind
}

#[cfg(windows)]
fn unique_partition(inventory: &Inventory, target: &Partition) -> bool {
    inventory
        .disks
        .iter()
        .flat_map(|disk| &disk.partitions)
        .filter(|partition| partition.id == target.id)
        .count()
        == 1
}

#[cfg(windows)]
pub fn discover(root: &Path, inventory: &Inventory) -> Resolution {
    match mounted_partition(root) {
        Ok(source) => resolve(&source, inventory),
        Err(error) => Resolution::Inaccessible(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn disk(number: u32, identity: &str) -> Disk {
        Disk {
            number: Some(number),
            identity: identity.into(),
            device_path: String::new(),
            capacity: Some(1024),
            bus: None,
            removable: None,
            read_only: Some(true),
            style: Some("GPT".into()),
            partitions: Vec::new(),
            errors: Vec::new(),
        }
    }

    #[test]
    fn requires_the_same_physical_disk_and_unique_complete_inventory() {
        let mut inventory = Inventory {
            disks: vec![disk(0, "gpt:windows"), disk(1, "gpt:foreign")],
            ..Default::default()
        };
        assert!(source_disk(&inventory, "gpt:windows", 0).is_ok());
        assert!(source_disk(&inventory, "gpt:windows", 1).is_err());

        inventory.disks.push(disk(2, "gpt:windows"));
        assert!(source_disk(&inventory, "gpt:windows", 0).is_err());

        inventory.disks.pop();
        inventory.disks[1].style = None;
        assert!(source_disk(&inventory, "gpt:windows", 0).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn distinct_partitions_cannot_be_treated_as_aliases() {
        let partition = |offset| Partition {
            number: 1,
            id: Some("11111111-2222-3333-4444-555555555555".into()),
            offset,
            length: 512,
            kind: ESP_TYPE.into(),
            efi: true,
        };
        let first = partition(512);
        assert!(same_partition(&first, &first));
        assert!(!same_partition(&first, &partition(1024)));

        let mut inventory = Inventory {
            disks: vec![disk(0, "gpt:windows")],
            ..Default::default()
        };
        inventory.disks[0].partitions.push(first.clone());
        assert!(unique_partition(&inventory, &first));
        inventory.disks[0].partitions.push(partition(1024));
        assert!(!unique_partition(&inventory, &first));
    }
}
