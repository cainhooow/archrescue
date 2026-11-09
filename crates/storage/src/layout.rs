//! Bounded parsing of Windows layout responses. No raw bytes are reinterpreted as Rust bools.
use crate::Partition;
use archrescue_core::{Error, ErrorKind, Result};
use std::mem::{offset_of, size_of};
use windows_sys::Win32::System::Ioctl::{
    DRIVE_LAYOUT_INFORMATION_EX, PARTITION_INFORMATION_EX, PARTITION_INFORMATION_GPT,
};
pub struct Layout {
    pub identity: Option<String>,
    pub style: String,
    pub partitions: Vec<Partition>,
}
fn field<const N: usize>(bytes: &[u8], offset: usize) -> Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or_else(|| Error::new(ErrorKind::Io, "Metadata offset overflow"))?;
    bytes
        .get(offset..end)
        .ok_or_else(|| Error::new(ErrorKind::Io, "Truncated device metadata"))?
        .try_into()
        .map_err(|_| Error::new(ErrorKind::Io, "Invalid metadata field"))
}
fn u32_at(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(field(b, o)?))
}
fn i64_at(b: &[u8], o: usize) -> Result<i64> {
    Ok(i64::from_le_bytes(field(b, o)?))
}
fn guid(b: &[u8], o: usize) -> Result<String> {
    let g = field::<16>(b, o)?;
    Ok(format!(
        "{:08x}-{:04x}-{:04x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        u32::from_le_bytes(g[..4].try_into().unwrap()),
        u16::from_le_bytes(g[4..6].try_into().unwrap()),
        u16::from_le_bytes(g[6..8].try_into().unwrap()),
        g[8],
        g[9],
        g[10],
        g[11],
        g[12],
        g[13],
        g[14],
        g[15]
    ))
}
pub fn parse(bytes: &[u8], capacity: Option<u64>) -> Result<Layout> {
    let style = u32_at(
        bytes,
        offset_of!(DRIVE_LAYOUT_INFORMATION_EX, PartitionStyle),
    )?;
    let count = u32_at(
        bytes,
        offset_of!(DRIVE_LAYOUT_INFORMATION_EX, PartitionCount),
    )? as usize;
    if count > 4096 {
        return Err(Error::new(
            ErrorKind::Unsupported,
            "Excessive partition count",
        ));
    }
    let union = offset_of!(DRIVE_LAYOUT_INFORMATION_EX, Anonymous);
    let mut result = Layout {
        identity: None,
        style: match style {
            0 => "MBR",
            1 => "GPT",
            _ => "RAW/unsupported",
        }
        .into(),
        partitions: vec![],
    };
    result.identity = match style {
        0 => Some(format!("mbr:{:08x}", u32_at(bytes, union)?)),
        1 => Some(format!("gpt:{}", guid(bytes, union)?)),
        _ => None,
    };
    for i in 0..count {
        let start = offset_of!(DRIVE_LAYOUT_INFORMATION_EX, PartitionEntry)
            + i * size_of::<PARTITION_INFORMATION_EX>();
        let entry = bytes
            .get(start..start + size_of::<PARTITION_INFORMATION_EX>())
            .ok_or_else(|| Error::new(ErrorKind::Io, "Truncated partition entry"))?;
        let entry_style = u32_at(entry, offset_of!(PARTITION_INFORMATION_EX, PartitionStyle))?;
        let offset = i64_at(entry, offset_of!(PARTITION_INFORMATION_EX, StartingOffset))?;
        let length = i64_at(entry, offset_of!(PARTITION_INFORMATION_EX, PartitionLength))?;
        if offset < 0 || length < 0 {
            return Err(Error::new(ErrorKind::Io, "Negative partition bounds"));
        }
        if length == 0 {
            continue;
        }
        if entry_style != style {
            return Err(Error::new(
                ErrorKind::Io,
                "Partition style differs from disk",
            ));
        }
        let end = (offset as u64)
            .checked_add(length as u64)
            .ok_or_else(|| Error::new(ErrorKind::Io, "Partition bounds overflow"))?;
        if capacity.is_some_and(|c| end > c) {
            return Err(Error::new(ErrorKind::Io, "Partition outside disk"));
        }
        let union = offset_of!(PARTITION_INFORMATION_EX, Anonymous);
        let kind = match entry_style {
            0 => format!("mbr:{:02x}", field::<1>(entry, union)?[0]),
            1 => guid(
                entry,
                union + offset_of!(PARTITION_INFORMATION_GPT, PartitionType),
            )?,
            _ => {
                return Err(Error::new(
                    ErrorKind::Unsupported,
                    "Unknown partition style",
                ))
            }
        };
        result.partitions.push(Partition {
            number: u32_at(entry, offset_of!(PARTITION_INFORMATION_EX, PartitionNumber))?,
            offset: offset as u64,
            length: length as u64,
            efi: kind == "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" || kind == "mbr:ef",
            kind,
        });
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<u8> {
        let mut b = vec![0u8; size_of::<DRIVE_LAYOUT_INFORMATION_EX>()];
        b[0..4].copy_from_slice(&1u32.to_le_bytes());
        b[4..8].copy_from_slice(&1u32.to_le_bytes());
        let start = offset_of!(DRIVE_LAYOUT_INFORMATION_EX, PartitionEntry);
        b[start..start + 4].copy_from_slice(&1u32.to_le_bytes());
        let offset = start + offset_of!(PARTITION_INFORMATION_EX, StartingOffset);
        b[offset..offset + 8].copy_from_slice(&4096i64.to_le_bytes());
        let length = start + offset_of!(PARTITION_INFORMATION_EX, PartitionLength);
        b[length..length + 8].copy_from_slice(&8192i64.to_le_bytes());
        let number = start + offset_of!(PARTITION_INFORMATION_EX, PartitionNumber);
        b[number..number + 4].copy_from_slice(&1u32.to_le_bytes());
        let gpt = start + offset_of!(PARTITION_INFORMATION_EX, Anonymous);
        b[gpt..gpt + 16].copy_from_slice(&[
            0x28, 0x73, 0x2a, 0xc1, 0x1f, 0xf8, 0xd2, 0x11, 0xba, 0x4b, 0, 0xa0, 0xc9, 0x3e, 0xc9,
            0x3b,
        ]);
        b
    }
    #[test]
    fn gpt_efi_layout_is_decoded_from_bytes() {
        let layout = parse(&fixture(), Some(16384)).unwrap();
        assert_eq!(layout.style, "GPT");
        assert_eq!(layout.partitions.len(), 1);
        assert!(layout.partitions[0].efi);
        assert_eq!(layout.partitions[0].length, 8192);
    }
    #[test]
    fn truncated_and_out_of_bounds_layout_rejected() {
        let b = fixture();
        for len in [0, 4, 8, 32, b.len() - 1] {
            assert!(parse(&b[..len], Some(16384)).is_err());
        }
        assert!(parse(&b, Some(8192)).is_err());
    }
    #[test]
    fn excessive_partition_count_rejected() {
        let mut b = fixture();
        b[4..8].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(parse(&b, None).is_err());
    }
    #[test]
    fn invalid_boolean_bytes_are_never_reinterpreted() {
        let mut b = fixture();
        let offset = offset_of!(DRIVE_LAYOUT_INFORMATION_EX, PartitionEntry)
            + offset_of!(PARTITION_INFORMATION_EX, RewritePartition);
        b[offset] = 255;
        assert!(parse(&b, Some(16384)).is_ok());
    }
}
