//! Windows API adapter. Handles are read-only; only query IOCTLs are used.

use std::{
    ffi::OsStr,
    mem::{offset_of, size_of},
    os::windows::ffi::{OsStrExt, OsStringExt},
    ptr::{null, null_mut},
};

use windows_sys::Win32::{
    Devices::DeviceAndDriverInstallation::*,
    Foundation::*,
    Storage::FileSystem::*,
    System::{Ioctl::*, IO::DeviceIoControl},
};

use super::*;

fn wide(text: impl AsRef<OsStr>) -> Vec<u16> {
    text.as_ref().encode_wide().chain(Some(0)).collect()
}

fn string(buffer: &[u16]) -> String {
    let end = buffer
        .iter()
        .position(|&unit| unit == 0)
        .unwrap_or(buffer.len());

    String::from_utf16_lossy(&buffer[..end])
}

fn error(context: &str) -> Error {
    Error::io(context, std::io::Error::last_os_error())
}

struct Handle(HANDLE);

impl Drop for Handle {
    fn drop(&mut self) {
        // SAFETY: Handle owns a valid CreateFileW handle.
        unsafe {
            CloseHandle(self.0);
        }
    }
}

fn open(path: &str) -> Result<Handle> {
    // SAFETY: terminated UTF-16 input; shared read access only; no create/truncate.
    let raw_handle = unsafe {
        CreateFileW(
            wide(path).as_ptr(),
            GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            0,
            null_mut(),
        )
    };

    if raw_handle == INVALID_HANDLE_VALUE {
        Err(error("Open read-only device"))
    } else {
        Ok(Handle(raw_handle))
    }
}

fn query(handle: &Handle, control_code: u32, input: &[u8]) -> Result<Vec<u8>> {
    // The cap bounds responses from damaged/unsupported drivers. Alignment is supplied by u64.
    for length in [4096usize, 65536, 1048576] {
        let mut buffer = vec![0u64; length / 8];
        let mut bytes_returned = 0;

        let input_ptr = if input.is_empty() {
            null()
        } else {
            input.as_ptr().cast()
        };

        // SAFETY: the query is synchronous and both buffers cover their supplied lengths.
        let succeeded = unsafe {
            DeviceIoControl(
                handle.0,
                control_code,
                input_ptr,
                input.len() as u32,
                buffer.as_mut_ptr().cast(),
                length as u32,
                &mut bytes_returned,
                null_mut(),
            )
        };

        if succeeded != 0 {
            if bytes_returned as usize > length {
                return Err(Error::new(ErrorKind::Io, "Invalid driver response size"));
            }

            // SAFETY: only initialized bytes within the allocated response are copied.
            let response = unsafe {
                std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), bytes_returned as usize)
            };

            return Ok(response.to_vec());
        }

        let os_error = std::io::Error::last_os_error();
        if !matches!(os_error.raw_os_error(), Some(122 | 234)) {
            return Err(Error::io("Read-only device query", os_error));
        }
    }

    Err(Error::new(
        ErrorKind::Unsupported,
        "Device response exceeds bounded buffer",
    ))
}

fn value<T: Copy>(data: &[u8], offset: usize) -> Result<T> {
    if offset
        .checked_add(size_of::<T>())
        .is_none_or(|end| end > data.len())
    {
        return Err(Error::new(ErrorKind::Io, "Truncated device metadata"));
    }

    // SAFETY: length checked; Windows query structs here contain only integers/unions/GUIDs.
    Ok(unsafe { std::ptr::read_unaligned(data.as_ptr().add(offset).cast::<T>()) })
}

fn extents(volume: &str) -> Result<Vec<u32>> {
    let mut disks: Vec<_> = volume_extents(volume)?
        .into_iter()
        .map(|extent| extent.DiskNumber)
        .collect();
    disks.sort();
    disks.dedup();

    Ok(disks)
}

fn volume_extents(volume: &str) -> Result<Vec<DISK_EXTENT>> {
    let handle = open(volume.trim_end_matches('\\'))?;
    let data = query(&handle, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, &[])?;
    let count = value::<u32>(&data, 0)? as usize;

    if count == 0 || count > 128 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Unsupported disk extent count",
        ));
    }

    let mut extents = Vec::new();
    for index in 0..count {
        extents.push(value(
            &data,
            offset_of!(VOLUME_DISK_EXTENTS, Extents) + index * size_of::<DISK_EXTENT>(),
        )?);
    }

    Ok(extents)
}

fn mount_and_volume(path: &Path) -> Result<(PathBuf, String)> {
    let mut mount_path = vec![0u16; 32768];
    let mut volume_name = vec![0u16; 1024];

    // SAFETY: both output buffers are writable and the path is NUL-terminated.
    unsafe {
        if GetVolumePathNameW(
            wide(path).as_ptr(),
            mount_path.as_mut_ptr(),
            mount_path.len() as u32,
        ) == 0
        {
            return Err(error("GetVolumePathNameW"));
        }

        if GetVolumeNameForVolumeMountPointW(
            mount_path.as_ptr(),
            volume_name.as_mut_ptr(),
            volume_name.len() as u32,
        ) == 0
        {
            return Err(error("GetVolumeNameForVolumeMountPointW"));
        }
    }

    let mount_path = std::ffi::OsString::from_wide(
        &mount_path[..mount_path
            .iter()
            .position(|&unit| unit == 0)
            .unwrap_or(mount_path.len())],
    );

    Ok((mount_path.into(), string(&volume_name)))
}

pub(crate) fn filesystem_name(path: &Path) -> Result<String> {
    mount_and_volume(path).map(|(_, volume)| volume)
}

pub fn identity(path: &Path) -> Result<VolumeIdentity> {
    let (_, volume) = mount_and_volume(path)?;
    let physical_disks = extents(&volume)?;

    Ok(VolumeIdentity {
        volume,
        physical_disks,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountedPartition {
    pub disk_number: u32,
    pub disk_id: String,
    pub style: String,
    pub partition: Partition,
    pub mount_path: PathBuf,
    pub volume_guid: String,
    pub filesystem: String,
    pub serial: u32,
}

pub fn mounted_partition(path: &Path) -> Result<MountedPartition> {
    let (mount_path, volume) = mount_and_volume(path)?;
    let extents = volume_extents(&volume)?;
    let [extent] = extents.as_slice() else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Persistent recovery requires a single partition extent",
        ));
    };

    let disk = open(&format!(r"\\.\PhysicalDrive{}", extent.DiskNumber))?;
    let layout = query(&disk, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, &[])?;
    let layout = crate::layout::parse(&layout, None)?;
    let disk_id = layout
        .identity
        .filter(|id| id != "mbr:00000000" && id != "gpt:00000000-0000-0000-0000-000000000000")
        .ok_or_else(|| Error::new(ErrorKind::Unsupported, "Missing on-disk partition table ID"))?;
    let partition = layout
        .partitions
        .iter()
        .find(|partition| {
            i64::try_from(partition.offset) == Ok(extent.StartingOffset)
                && i64::try_from(partition.length) == Ok(extent.ExtentLength)
        })
        .ok_or_else(|| Error::new(ErrorKind::Unsupported, "Volume is not a basic partition"))?;

    let mut serial = 0;
    let mut filesystem = [0u16; 256];
    // SAFETY: volume is terminated and both requested outputs are writable.
    if unsafe {
        GetVolumeInformationW(
            wide(&volume).as_ptr(),
            null_mut(),
            0,
            &mut serial,
            null_mut(),
            null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    } == 0
    {
        return Err(error("Read filesystem serial for recovery"));
    }

    Ok(MountedPartition {
        disk_number: extent.DiskNumber,
        disk_id,
        style: layout.style,
        partition: partition.clone(),
        mount_path: safe_existing(&mount_path)?,
        volume_guid: volume,
        filesystem: string(&filesystem),
        serial,
    })
}

pub fn recovery_location(path: &Path) -> Result<RecoveryLocation> {
    let mounted = mounted_partition(path)?;
    let partition = mounted.partition;
    let directory = path.strip_prefix(mounted.mount_path).map_err(|_| {
        Error::new(
            ErrorKind::Validation,
            "Cannot identify path relative to volume",
        )
    })?;

    Ok(RecoveryLocation {
        filesystem_id: format!(
            "{};{};{};{};{:08x};{}",
            mounted.disk_id,
            partition.id.as_deref().unwrap_or(&partition.kind),
            partition.offset,
            partition.length,
            mounted.serial,
            mounted.filesystem
        ),
        directory: directory.into(),
    })
}

/// Read the filesystem dirty flags without opening the device for write access.
pub fn require_clean_volume(path: &Path) -> Result<()> {
    let (_, volume) = mount_and_volume(path)?;
    let handle = open(volume.trim_end_matches('\\'))?;
    let data = query(&handle, FSCTL_IS_VOLUME_DIRTY, &[])?;
    if value::<u32>(&data, 0)? != 0 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Filesystem dirty flags require independent investigation",
        ));
    }

    Ok(())
}

pub fn space(path: &Path) -> Result<Space> {
    let (mut available, mut total, mut free) = (0, 0, 0);

    if unsafe { GetDiskFreeSpaceExW(wide(path).as_ptr(), &mut available, &mut total, &mut free) }
        == 0
    {
        return Err(error("GetDiskFreeSpaceExW"));
    }

    Space::checked(total, free, available)
}

fn disk(path: String) -> Disk {
    let mut disk = Disk {
        number: None,
        identity: path.clone(),
        device_path: path.clone(),
        capacity: None,
        bus: None,
        removable: None,
        read_only: None,
        style: None,
        partitions: vec![],
        errors: vec![],
    };

    let handle = match open(&path) {
        Ok(handle) => handle,
        Err(error) => {
            disk.errors.push(error);
            return disk;
        }
    };
    match query(&handle, IOCTL_STORAGE_GET_DEVICE_NUMBER, &[])
        .and_then(|data| value::<STORAGE_DEVICE_NUMBER>(&data, 0))
    {
        Ok(number) => disk.number = Some(number.DeviceNumber),
        Err(error) => disk.errors.push(error),
    }

    let length =
        query(&handle, IOCTL_DISK_GET_LENGTH_INFO, &[]).and_then(|bytes| value::<i64>(&bytes, 0));

    match length {
        Ok(length) if length >= 0 => disk.capacity = Some(length as u64),
        Ok(_) => disk
            .errors
            .push(Error::new(ErrorKind::Io, "Negative disk length")),
        Err(error) => disk.errors.push(error),
    }

    // StorageDeviceProperty / PropertyStandardQuery; no serial number is read or retained.
    match query(&handle, IOCTL_STORAGE_QUERY_PROPERTY, &[0; 12]) {
        Ok(bytes) => {
            match value::<u32>(&bytes, offset_of!(STORAGE_DEVICE_DESCRIPTOR, BusType)) {
                Ok(bus) => disk.bus = Some(bus),
                Err(error) => disk.errors.push(error),
            }

            match bytes.get(offset_of!(STORAGE_DEVICE_DESCRIPTOR, RemovableMedia)) {
                Some(0) => disk.removable = Some(false),
                Some(1) => disk.removable = Some(true),
                _ => disk.errors.push(Error::new(
                    ErrorKind::Io,
                    "Invalid removable-media metadata",
                )),
            }
        }
        Err(error) => disk.errors.push(error),
    }

    disk.read_only = match query(&handle, IOCTL_DISK_IS_WRITABLE, &[]) {
        Ok(_) => Some(false),
        Err(error) if error.os_code == Some(ERROR_WRITE_PROTECT as i32) => Some(true),
        Err(_) => None,
    };

    if let Err(error) = read_layout(&handle, &mut disk) {
        disk.errors.push(error);
    }

    disk
}

fn read_layout(handle: &Handle, disk: &mut Disk) -> Result<()> {
    let bytes = query(handle, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, &[])?;
    let parsed = crate::layout::parse(&bytes, disk.capacity)?;

    if let Some(id) = parsed.identity {
        disk.identity = if parsed.style == "MBR" {
            format!("{id};{}", disk.device_path)
        } else {
            id
        };
    }

    disk.style = Some(parsed.style);
    disk.partitions = parsed.partitions;

    Ok(())
}

pub fn inventory() -> Inventory {
    let mut result = Inventory::default();

    enumerate_disks(&mut result);
    enumerate_volumes(&mut result);

    result
}

fn enumerate_disks(result: &mut Inventory) {
    // SAFETY: documented SetupAPI enumeration with bounded detail buffers and a released device set.
    unsafe {
        let device_set = SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_DISK,
            null(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );

        if device_set == -1 {
            result.errors.push(error("SetupDiGetClassDevsW"));
            return;
        }

        let mut index = 0;
        loop {
            let mut interface = SP_DEVICE_INTERFACE_DATA {
                cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                ..Default::default()
            };

            if SetupDiEnumDeviceInterfaces(
                device_set,
                null(),
                &GUID_DEVINTERFACE_DISK,
                index,
                &mut interface,
            ) == 0
            {
                if GetLastError() != ERROR_NO_MORE_ITEMS {
                    result.errors.push(error("SetupDiEnumDeviceInterfaces"));
                }
                break;
            }

            index += 1;

            let mut required_bytes = 0;
            SetupDiGetDeviceInterfaceDetailW(
                device_set,
                &interface,
                null_mut(),
                0,
                &mut required_bytes,
                null_mut(),
            );

            if !(8..=65536).contains(&required_bytes) {
                result
                    .errors
                    .push(Error::new(ErrorKind::Io, "Invalid device interface length"));
                continue;
            }

            let mut detail_buffer = vec![0u64; (required_bytes as usize).div_ceil(8)];
            let detail = detail_buffer
                .as_mut_ptr()
                .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
            (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;

            if SetupDiGetDeviceInterfaceDetailW(
                device_set,
                &interface,
                detail,
                required_bytes,
                null_mut(),
                null_mut(),
            ) == 0
            {
                result
                    .errors
                    .push(error("SetupDiGetDeviceInterfaceDetailW"));
                continue;
            }

            let path = std::slice::from_raw_parts(
                std::ptr::addr_of!((*detail).DevicePath).cast::<u16>(),
                (required_bytes as usize - 4) / 2,
            );
            result.disks.push(disk(string(path)));
        }

        SetupDiDestroyDeviceInfoList(device_set);
    }
}

fn enumerate_volumes(result: &mut Inventory) {
    // SAFETY: volume enumeration writes to the supplied buffer and releases the search handle.
    unsafe {
        let mut volume_name = vec![0u16; 1024];
        let find_handle = FindFirstVolumeW(volume_name.as_mut_ptr(), volume_name.len() as u32);

        if find_handle == INVALID_HANDLE_VALUE {
            result.errors.push(error("FindFirstVolumeW"));
            return;
        }

        loop {
            // Successful enumeration supplies a NUL-terminated volume name.
            result.volumes.push(read_volume(&volume_name));

            if FindNextVolumeW(
                find_handle,
                volume_name.as_mut_ptr(),
                volume_name.len() as u32,
            ) == 0
            {
                if GetLastError() != ERROR_NO_MORE_FILES {
                    result.errors.push(error("FindNextVolumeW"));
                }
                break;
            }
        }

        FindVolumeClose(find_handle);
    }
}

// The caller supplies a NUL-terminated volume name from Win32 enumeration.
unsafe fn read_volume(volume_name: &[u16]) -> Volume {
    let volume_id = string(volume_name);
    let mut volume = Volume {
        identity: volume_id.clone(),
        paths: vec![],
        filesystem: None,
        space: None,
        physical_disks: vec![],
        errors: vec![],
    };

    let mut paths = vec![0u16; 32768];
    let mut required = 0;
    if GetVolumePathNamesForVolumeNameW(
        volume_name.as_ptr(),
        paths.as_mut_ptr(),
        paths.len() as u32,
        &mut required,
    ) != 0
    {
        volume.paths = paths
            .split(|unit| *unit == 0)
            .take_while(|units| !units.is_empty())
            .map(|units| PathBuf::from(std::ffi::OsString::from_wide(units)))
            .collect();
    } else {
        volume
            .errors
            .push(error("GetVolumePathNamesForVolumeNameW"));
    }

    let mut filesystem_name = vec![0u16; 256];
    if GetVolumeInformationW(
        volume_name.as_ptr(),
        null_mut(),
        0,
        null_mut(),
        null_mut(),
        null_mut(),
        filesystem_name.as_mut_ptr(),
        filesystem_name.len() as u32,
    ) != 0
    {
        volume.filesystem = Some(string(&filesystem_name));
    } else {
        volume.errors.push(error("GetVolumeInformationW"));
    }

    match space(Path::new(&volume_id)) {
        Ok(space) => volume.space = Some(space),
        Err(error) => volume.errors.push(error),
    }

    match extents(&volume_id) {
        Ok(disks) => volume.physical_disks = disks,
        Err(error) => volume.errors.push(error),
    }

    volume
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum DiskHealth {
    NoFailurePredicted,
    FailurePredicted,
    Unknown,
}

pub fn disk_health(device_path: &str) -> DiskHealth {
    let prediction = open(device_path)
        .and_then(|handle| query(&handle, IOCTL_STORAGE_PREDICT_FAILURE, &[]))
        .and_then(|data| value::<u32>(&data, 0));

    match prediction {
        Ok(0) => DiskHealth::NoFailurePredicted,
        Ok(_) => DiskHealth::FailurePredicted,
        Err(_) => DiskHealth::Unknown,
    }
}

pub fn require_writable_disk(device_path: &str) -> Result<()> {
    let disk = open(device_path)?;
    query(&disk, IOCTL_DISK_IS_WRITABLE, &[])?;

    Ok(())
}

/// WinPE FAT32 roots can fail DOS-name normalization while GUID normalization succeeds.
/// Every path component was already inspected by safe_existing before this narrow recovery.
pub(crate) fn canonicalize_efi_root(path: &Path) -> Result<PathBuf> {
    let (mount, volume) = mount_and_volume(path)?;
    let extents_before = volume_extents(&volume)?;
    let [extent] = extents_before.as_slice() else {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "EFI root requires one disk extent",
        ));
    };
    let disk = open(&format!(r"\\.\PhysicalDrive{}", extent.DiskNumber))?;
    let layout = crate::layout::parse(&query(&disk, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, &[])?, None)?;
    if layout.style != "GPT"
        || layout
            .identity
            .as_deref()
            .is_none_or(|id| id == "gpt:00000000-0000-0000-0000-000000000000")
        || !layout.partitions.iter().any(|partition| {
            partition.efi
                && i64::try_from(partition.offset) == Ok(extent.StartingOffset)
                && i64::try_from(partition.length) == Ok(extent.ExtentLength)
        })
    {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Normalization recovery requires an identified GPT ESP",
        ));
    }

    let mut filesystem = [0u16; 256];
    // SAFETY: volume is terminated and filesystem has the supplied writable capacity.
    if unsafe {
        GetVolumeInformationW(
            wide(&volume).as_ptr(),
            null_mut(),
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            filesystem.as_mut_ptr(),
            filesystem.len() as u32,
        )
    } == 0
    {
        return Err(error("Inspect EFI filesystem for root normalization"));
    }
    if string(&filesystem) != "FAT32" {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Normalization recovery requires FAT32",
        ));
    }

    // SAFETY: path is terminated. This opens the directory itself with read-only access.
    let raw = unsafe {
        CreateFileW(
            wide(path).as_ptr(),
            FILE_READ_ATTRIBUTES,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            null(),
            OPEN_EXISTING,
            FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
            null_mut(),
        )
    };
    if raw == INVALID_HANDLE_VALUE {
        return Err(error("Open EFI root for GUID normalization"));
    }
    let handle = Handle(raw);
    let mut attributes = BY_HANDLE_FILE_INFORMATION::default();
    // SAFETY: handle is owned and attributes provides writable file information.
    // WinPE FAT32 can reject FileAttributeTagInfo. Basic handle information
    // still reports the directory, reparse and encryption attribute bits.
    if unsafe { GetFileInformationByHandle(handle.0, &mut attributes) } == 0 {
        return Err(error("Recheck EFI root attributes"));
    }
    if attributes.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY == 0
        || attributes.dwFileAttributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_ENCRYPTED)
            != 0
    {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "EFI root attributes changed",
        ));
    }

    let mut normalized = vec![0u16; 32768];
    // SAFETY: handle remains live and normalized holds the supplied capacity.
    // VOLUME_NAME_GUID preserves volume identity when WinPE cannot normalize a DOS root.
    let length = unsafe {
        GetFinalPathNameByHandleW(
            handle.0,
            normalized.as_mut_ptr(),
            normalized.len() as u32,
            VOLUME_NAME_GUID,
        )
    };
    if length == 0 || length as usize >= normalized.len() {
        return Err(error("Normalize EFI root by volume GUID"));
    }
    let normalized = String::from_utf16_lossy(&normalized[..length as usize]);
    let extents_after = volume_extents(&volume)?;
    let same_extent = extents_after.as_slice().first().is_some_and(|after| {
        extents_after.len() == 1
            && after.DiskNumber == extent.DiskNumber
            && after.StartingOffset == extent.StartingOffset
            && after.ExtentLength == extent.ExtentLength
    });
    if !normalized
        .trim_end_matches('\\')
        .eq_ignore_ascii_case(volume.trim_end_matches('\\'))
        || !same_extent
        || mount_and_volume(path)?.1 != volume
    {
        return Err(Error::new(
            ErrorKind::Integrity,
            "EFI root volume identity changed",
        ));
    }

    let mount = mount.to_string_lossy();
    let canonical = if mount.starts_with(r"\\?\") {
        PathBuf::from(mount.as_ref())
    } else {
        PathBuf::from(format!(r"\\?\{mount}"))
    };
    if canonical.components().count() != 2 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "EFI root path is not a volume root",
        ));
    }

    Ok(canonical)
}
