//! Windows API adapter. Handles are read-only; only query IOCTLs are used.
use super::*;
use std::{
    ffi::OsStr,
    mem::{offset_of, size_of},
    os::windows::ffi::{OsStrExt, OsStringExt},
    ptr::{null, null_mut},
};
use windows_sys::{
    core::GUID,
    Win32::{
        Devices::DeviceAndDriverInstallation::*,
        Foundation::*,
        Storage::FileSystem::*,
        System::{Ioctl::*, IO::DeviceIoControl},
    },
};
fn wide(s: impl AsRef<OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}
fn string(s: &[u16]) -> String {
    String::from_utf16_lossy(&s[..s.iter().position(|&c| c == 0).unwrap_or(s.len())])
}
fn error(context: &str) -> Error {
    Error::io(context, std::io::Error::last_os_error())
}
struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn open(path: &str) -> Result<Handle> {
    // SAFETY: terminated UTF-16 input; shared read access only; no create/truncate.
    let h = unsafe {
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
    if h == INVALID_HANDLE_VALUE {
        Err(error("Open read-only device"))
    } else {
        Ok(Handle(h))
    }
}
fn query(h: &Handle, code: u32, input: &[u8]) -> Result<Vec<u8>> {
    // The cap bounds responses from damaged/unsupported drivers. Alignment is supplied by u64.
    for length in [4096usize, 65536, 1048576] {
        let mut buffer = vec![0u64; length / 8];
        let mut returned = 0;
        let ok = unsafe {
            DeviceIoControl(
                h.0,
                code,
                if input.is_empty() {
                    null()
                } else {
                    input.as_ptr().cast()
                },
                input.len() as u32,
                buffer.as_mut_ptr().cast(),
                length as u32,
                &mut returned,
                null_mut(),
            )
        };
        if ok != 0 {
            if returned as usize > length {
                return Err(Error::new(ErrorKind::Io, "Invalid driver response size"));
            }
            // SAFETY: only initialized bytes within the allocated response are copied.
            return Ok(unsafe {
                std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), returned as usize)
            }
            .to_vec());
        }
        let err = std::io::Error::last_os_error();
        if !matches!(err.raw_os_error(), Some(122 | 234)) {
            return Err(Error::io("Read-only device query", err));
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
        .is_none_or(|n| n > data.len())
    {
        return Err(Error::new(ErrorKind::Io, "Truncated device metadata"));
    }
    // SAFETY: length checked; Windows query structs here contain only integers/unions/GUIDs.
    Ok(unsafe { std::ptr::read_unaligned(data.as_ptr().add(offset).cast::<T>()) })
}
fn guid(g: GUID) -> String {
    format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        g.data1,
        g.data2,
        g.data3,
        g.data4[0],
        g.data4[1],
        g.data4[2],
        g.data4[3],
        g.data4[4],
        g.data4[5],
        g.data4[6],
        g.data4[7]
    )
}
fn extents(volume: &str) -> Result<Vec<u32>> {
    let h = open(volume.trim_end_matches('\\'))?;
    let data = query(&h, IOCTL_VOLUME_GET_VOLUME_DISK_EXTENTS, &[])?;
    let count = value::<u32>(&data, 0)? as usize;
    if count == 0 || count > 128 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Unsupported disk extent count",
        ));
    }
    let mut disks = Vec::new();
    for i in 0..count {
        let e: DISK_EXTENT = value(
            &data,
            offset_of!(VOLUME_DISK_EXTENTS, Extents) + i * size_of::<DISK_EXTENT>(),
        )?;
        disks.push(e.DiskNumber);
    }
    disks.sort();
    disks.dedup();
    Ok(disks)
}
pub fn identity(path: &Path) -> Result<VolumeIdentity> {
    let mut mount = vec![0u16; 32768];
    let mut name = vec![0u16; 1024];
    unsafe {
        if GetVolumePathNameW(wide(path).as_ptr(), mount.as_mut_ptr(), mount.len() as u32) == 0 {
            return Err(error("GetVolumePathNameW"));
        }
        if GetVolumeNameForVolumeMountPointW(mount.as_ptr(), name.as_mut_ptr(), name.len() as u32)
            == 0
        {
            return Err(error("GetVolumeNameForVolumeMountPointW"));
        }
    }
    let volume = string(&name);
    let physical_disks = extents(&volume)?;
    Ok(VolumeIdentity {
        volume,
        physical_disks,
    })
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
    let mut d = Disk {
        identity: path.clone(),
        device_path: path.clone(),
        capacity: None,
        bus: None,
        removable: None,
        style: None,
        partitions: vec![],
        errors: vec![],
    };
    let h = match open(&path) {
        Ok(h) => h,
        Err(e) => {
            d.errors.push(e);
            return d;
        }
    };
    match query(&h, IOCTL_DISK_GET_LENGTH_INFO, &[]).and_then(|v| value::<i64>(&v, 0)) {
        Ok(n) if n >= 0 => d.capacity = Some(n as u64),
        Ok(_) => d
            .errors
            .push(Error::new(ErrorKind::Io, "Negative disk length")),
        Err(e) => d.errors.push(e),
    }
    // StorageDeviceProperty / PropertyStandardQuery; no serial number is read or retained.
    match query(&h, IOCTL_STORAGE_QUERY_PROPERTY, &[0; 12])
        .and_then(|v| value::<STORAGE_DEVICE_DESCRIPTOR>(&v, 0))
    {
        Ok(s) => {
            d.bus = Some(s.BusType as u32);
            d.removable = Some(s.RemovableMedia);
        }
        Err(e) => d.errors.push(e),
    }
    if let Err(e) = read_layout(&h, &mut d) {
        d.errors.push(e);
    }
    d
}
fn read_layout(h: &Handle, d: &mut Disk) -> Result<()> {
    let bytes = query(h, IOCTL_DISK_GET_DRIVE_LAYOUT_EX, &[])?;
    let layout: DRIVE_LAYOUT_INFORMATION_EX = value(&bytes, 0)?;
    d.style = Some(
        match layout.PartitionStyle as i32 {
            PARTITION_STYLE_GPT => "GPT",
            PARTITION_STYLE_MBR => "MBR",
            _ => "RAW/unsupported",
        }
        .into(),
    );
    unsafe {
        match layout.PartitionStyle as i32 {
            PARTITION_STYLE_GPT => {
                d.identity = format!("gpt:{}", guid(layout.Anonymous.Gpt.DiskId))
            }
            PARTITION_STYLE_MBR => {
                d.identity = format!(
                    "mbr:{:08x};{}",
                    layout.Anonymous.Mbr.Signature, d.device_path
                )
            }
            _ => {}
        }
    }
    if layout.PartitionCount > 4096 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Excessive partition count",
        ));
    }
    for i in 0..layout.PartitionCount as usize {
        let p: PARTITION_INFORMATION_EX = value(
            &bytes,
            offset_of!(DRIVE_LAYOUT_INFORMATION_EX, PartitionEntry)
                + i * size_of::<PARTITION_INFORMATION_EX>(),
        )?;
        if p.StartingOffset < 0 || p.PartitionLength < 0 {
            return Err(Error::new(ErrorKind::Io, "Invalid partition bounds"));
        }
        if p.PartitionLength == 0 {
            continue;
        }
        let end = (p.StartingOffset as u64)
            .checked_add(p.PartitionLength as u64)
            .ok_or_else(|| Error::new(ErrorKind::Io, "Partition bounds overflow"))?;
        if d.capacity.is_some_and(|n| end > n) {
            return Err(Error::new(ErrorKind::Io, "Partition outside disk"));
        }
        let kind = unsafe {
            match p.PartitionStyle {
                PARTITION_STYLE_GPT => guid(p.Anonymous.Gpt.PartitionType),
                PARTITION_STYLE_MBR => format!("mbr:{:02x}", p.Anonymous.Mbr.PartitionType),
                _ => "unsupported".into(),
            }
        };
        let efi = kind == "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" || kind == "mbr:ef";
        d.partitions.push(Partition {
            number: p.PartitionNumber,
            offset: p.StartingOffset as u64,
            length: p.PartitionLength as u64,
            kind,
            efi,
        });
    }
    Ok(())
}
pub fn inventory() -> Inventory {
    let mut result = Inventory::default();
    // SAFETY: documented SetupAPI enumeration with bounded detail buffers and released set.
    unsafe {
        let set = SetupDiGetClassDevsW(
            &GUID_DEVINTERFACE_DISK,
            null(),
            null_mut(),
            DIGCF_PRESENT | DIGCF_DEVICEINTERFACE,
        );
        if set == -1 {
            result.errors.push(error("SetupDiGetClassDevsW"));
        } else {
            let mut index = 0;
            loop {
                let mut interface = SP_DEVICE_INTERFACE_DATA {
                    cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
                    ..Default::default()
                };
                if SetupDiEnumDeviceInterfaces(
                    set,
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
                let mut required = 0;
                SetupDiGetDeviceInterfaceDetailW(
                    set,
                    &interface,
                    null_mut(),
                    0,
                    &mut required,
                    null_mut(),
                );
                if !(8..=65536).contains(&required) {
                    result
                        .errors
                        .push(Error::new(ErrorKind::Io, "Invalid device interface length"));
                    continue;
                }
                let mut buf = vec![0u64; (required as usize).div_ceil(8)];
                let detail = buf.as_mut_ptr().cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
                (*detail).cbSize = size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
                if SetupDiGetDeviceInterfaceDetailW(
                    set,
                    &interface,
                    detail,
                    required,
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
                    (required as usize - 4) / 2,
                );
                result.disks.push(disk(string(path)));
            }
            SetupDiDestroyDeviceInfoList(set);
        }
        let mut name = vec![0u16; 1024];
        let find = FindFirstVolumeW(name.as_mut_ptr(), name.len() as u32);
        if find == INVALID_HANDLE_VALUE {
            result.errors.push(error("FindFirstVolumeW"));
            return result;
        }
        loop {
            let id = string(&name);
            let mut v = Volume {
                identity: id.clone(),
                paths: vec![],
                filesystem: None,
                space: None,
                physical_disks: vec![],
                errors: vec![],
            };
            let mut paths = vec![0u16; 32768];
            let mut required = 0;
            if GetVolumePathNamesForVolumeNameW(
                name.as_ptr(),
                paths.as_mut_ptr(),
                paths.len() as u32,
                &mut required,
            ) != 0
            {
                v.paths = paths
                    .split(|c| *c == 0)
                    .take_while(|s| !s.is_empty())
                    .map(|s| PathBuf::from(std::ffi::OsString::from_wide(s)))
                    .collect();
            } else {
                v.errors.push(error("GetVolumePathNamesForVolumeNameW"));
            }
            let mut fsname = vec![0u16; 256];
            if GetVolumeInformationW(
                name.as_ptr(),
                null_mut(),
                0,
                null_mut(),
                null_mut(),
                null_mut(),
                fsname.as_mut_ptr(),
                fsname.len() as u32,
            ) != 0
            {
                v.filesystem = Some(string(&fsname));
            } else {
                v.errors.push(error("GetVolumeInformationW"));
            }
            match space(Path::new(&id)) {
                Ok(s) => v.space = Some(s),
                Err(e) => v.errors.push(e),
            }
            match extents(&id) {
                Ok(d) => v.physical_disks = d,
                Err(e) => v.errors.push(e),
            }
            result.volumes.push(v);
            if FindNextVolumeW(find, name.as_mut_ptr(), name.len() as u32) == 0 {
                if GetLastError() != ERROR_NO_MORE_FILES {
                    result.errors.push(error("FindNextVolumeW"));
                }
                break;
            }
        }
        FindVolumeClose(find);
    }
    result
}
