//! Host-issued, single-operation authorization for a disposable VM.

use archrescue_core::{Error, ErrorKind, Result};
use archrescue_storage::Disk;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::{Deserialize, Serialize};

use crate::{ImageType, Intent, Plan, Qualification};

#[cfg(all(windows, target_arch = "x86_64"))]
pub mod windows;

const MIB: u64 = 1024 * 1024;
const MAX_AUTHORIZATION_SECONDS: u64 = 3600;
const PURPOSE: &str = "archrescue-disposable-uefi-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Layout {
    pub efi_mib: u64,
    pub msr_mib: u64,
    pub windows_offset_mib: u64,
    pub minimum_windows_bytes: u64,
}

impl Layout {
    pub(crate) fn for_image(minimum_windows_bytes: u64) -> Self {
        // Microsoft UEFI/GPT requirements: FAT32 ESP (200 MiB on 512/512e, 300 MiB on 4Kn)
        // and a 16 MiB MSR. This isolated profile reserves 300 MiB and uses 1 MiB alignment.
        let efi_mib = 300;
        let msr_mib = 16;

        Self {
            efi_mib,
            msr_mib,
            windows_offset_mib: 1 + efi_mib + msr_mib,
            minimum_windows_bytes,
        }
    }

    pub(crate) fn required_disk_bytes(&self) -> Option<u64> {
        self.windows_offset_mib
            .checked_add(2)?
            .checked_mul(MIB)?
            .checked_add(self.minimum_windows_bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registration {
    pub schema_version: u32,
    pub purpose: String,
    pub operation_id: String,
    pub issued_unix: u64,
    pub expires_unix: u64,
    pub plan_fingerprint: String,
    pub vm_uuid: String,
    pub target_vdi_uuid: String,
    pub target_backing_file: String,
    pub controller: String,
    pub port: u8,
    pub device: u8,
    pub target_gpt_id: String,
    pub target_serial: String,
    pub target_capacity: u64,
    pub initial_layout: String,
    pub source_gpt_id: String,
    pub backup_gpt_id: String,
    pub evidence_gpt_id: String,
    pub backup_operation: String,
    pub backup_manifest_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub serial: String,
    pub product: String,
    pub bus: u32,
    pub logical_sector: u32,
    pub physical_sector: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreBinding {
    pub recovery_operation: String,
    pub deployment_plan_sha256: String,
    pub backup_operation: String,
    pub backup_manifest_sha256: String,
}

fn permission(reason: &str) -> Error {
    Error::new(ErrorKind::Permission, reason)
}

fn decode_hex<const N: usize>(input: &str) -> Result<[u8; N]> {
    let mut output = [0; N];
    if input.len() != N * 2 || !input.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(permission(
            "Invalid laboratory signature or public key encoding",
        ));
    }

    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&input[index * 2..index * 2 + 2], 16)
            .map_err(|_| permission("Invalid laboratory signature encoding"))?;
    }

    Ok(output)
}

fn is_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if matches!(index, 8 | 13 | 18 | 23) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
        && value != "00000000-0000-0000-0000-000000000000"
}

fn is_gpt(value: &str) -> bool {
    value.strip_prefix("gpt:").is_some_and(is_uuid)
}

pub fn authenticate(payload: &[u8], signature: &str, public_key: &str) -> Result<Registration> {
    if payload.len() > 16 * 1024 {
        return Err(permission("Laboratory registration exceeds its size limit"));
    }

    let key = VerifyingKey::from_bytes(&decode_hex(public_key)?)
        .map_err(|_| permission("Invalid laboratory public key"))?;
    let signature = Signature::from_bytes(&decode_hex(signature.trim())?);
    key.verify_strict(payload, &signature)
        .map_err(|_| permission("Laboratory registration signature rejected"))?;

    serde_json::from_slice(payload)
        .map_err(|error| Error::new(ErrorKind::Validation, error.to_string()))
}

impl Registration {
    pub fn validate<'a>(&self, plan: &'a Plan, confirmation: &str, now: u64) -> Result<&'a Layout> {
        plan.validate_fingerprint()?;
        let layout = plan.lab_layout.as_ref().ok_or_else(|| {
            permission("The reviewed plan does not contain a disposable GPT layout")
        })?;

        if self.schema_version != 1
            || self.purpose != PURPOSE
            || !is_uuid(&self.operation_id)
            || !is_uuid(&self.vm_uuid)
            || !is_uuid(&self.target_vdi_uuid)
            || !self.target_backing_file.starts_with('/')
            || self.controller != "SATA"
            || self.device != 0
            || self.port >= 30
            || self.initial_layout != "empty-gpt"
            || self.target_serial.len() != 20
            || !self
                .target_serial
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
            || !self.backup_operation.starts_with("ar-")
            || self.backup_operation.len() > 100
            || !self
                .backup_operation
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit() || b"r-".contains(&byte))
            || decode_hex::<32>(&self.backup_manifest_sha256).is_err()
        {
            return Err(permission("Invalid disposable laboratory registration"));
        }

        let mut disk_ids = [
            self.target_gpt_id.to_ascii_lowercase(),
            self.source_gpt_id.to_ascii_lowercase(),
            self.backup_gpt_id.to_ascii_lowercase(),
            self.evidence_gpt_id.to_ascii_lowercase(),
        ];
        disk_ids.sort();
        if !disk_ids.iter().all(|id| is_gpt(id))
            || disk_ids.windows(2).any(|pair| pair[0] == pair[1])
        {
            return Err(permission(
                "Target, source, backup and evidence must use distinct GPT disks",
            ));
        }

        if self.issued_unix > now
            || self.expires_unix <= now
            || self.expires_unix.saturating_sub(self.issued_unix) > MAX_AUTHORIZATION_SECONDS
            || self.plan_fingerprint != plan.fingerprint
            || confirmation != self.confirmation()
        {
            return Err(permission("Expired or mismatched deployment authorization"));
        }

        if plan.intent != Intent::Clean
            || plan.result != Qualification::Supported
            || plan.target.firmware != "UEFI"
            || plan.target.writable != Some(true)
            || !plan.target.volumes.is_empty()
            || !plan.target.discovery.installations.is_empty()
            || plan.target.preservation.is_some()
            || plan.media.image_type != ImageType::Wim
            || plan.selected_image.installation.as_deref() != Some("Client")
            || plan.selected_image.architecture.as_deref() != Some("x64")
            || plan.selected_image.index == 0
            || !plan.media.images.contains(&plan.selected_image)
            || plan.minimum_bytes.map(Layout::for_image).as_ref() != Some(layout)
            || !layout
                .required_disk_bytes()
                .is_some_and(|size| size <= self.target_capacity)
        {
            return Err(permission(
                "Plan falls outside the disposable Windows client profile",
            ));
        }

        self.verify_empty_disk(&plan.target.disk)?;

        Ok(layout)
    }

    pub fn confirmation(&self) -> String {
        format!(
            "AUTORIZAR-LAB:{}:{}",
            self.operation_id, self.plan_fingerprint
        )
    }

    pub(crate) fn verify_empty_disk(&self, disk: &Disk) -> Result<()> {
        if disk.number.is_none()
            || !disk.identity.eq_ignore_ascii_case(&self.target_gpt_id)
            || disk.style.as_deref() != Some("GPT")
            || disk.capacity != Some(self.target_capacity)
            || disk.removable != Some(false)
            || disk.read_only != Some(false)
            || !disk.errors.is_empty()
            || !disk.partitions.is_empty()
        {
            return Err(permission(
                "The registered empty writable GPT disk no longer matches",
            ));
        }

        Ok(())
    }

    #[cfg(all(windows, target_arch = "x86_64"))]
    fn verify_attachment(&self, vm_uuid: &str, attachment: &Attachment) -> Result<()> {
        // SATA is STORAGE_BUS_TYPE BusTypeSata. VirtualBox may expose ATA on its AHCI driver.
        if !self.vm_uuid.eq_ignore_ascii_case(vm_uuid)
            || !self.target_serial.eq_ignore_ascii_case(&attachment.serial)
            || !attachment
                .product
                .trim()
                .eq_ignore_ascii_case("VBOX HARDDISK")
            || !matches!(attachment.bus, 3 | 11)
            || attachment.logical_sector != 512
            || attachment.physical_sector != 512
        {
            return Err(permission(
                "Guest VM or SATA device identity differs from the host registration",
            ));
        }

        Ok(())
    }

    pub fn restore_binding(&self) -> RestoreBinding {
        RestoreBinding {
            recovery_operation: self.operation_id.clone(),
            deployment_plan_sha256: self.plan_fingerprint.clone(),
            backup_operation: self.backup_operation.clone(),
            backup_manifest_sha256: self.backup_manifest_sha256.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::{Signer, SigningKey};

    use super::*;

    fn fixture() -> (Plan, Registration) {
        let (media, target) = crate::plan::tests::fixture();
        let plan = crate::plan::build(media, target, 1, Intent::Clean).unwrap();
        let registration = Registration {
            schema_version: 1,
            purpose: PURPOSE.into(),
            operation_id: "c1f87340-11e3-4b1b-866b-7b641613b624".into(),
            issued_unix: 100,
            expires_unix: 200,
            plan_fingerprint: plan.fingerprint.clone(),
            vm_uuid: "50bd4ff9-4cc2-4a72-8fe8-2604c2ea228f".into(),
            target_vdi_uuid: "f9cbe157-0c7f-4cee-adb2-141f4faef6c2".into(),
            target_backing_file: "/lab/run/target.vdi".into(),
            controller: "SATA".into(),
            port: 2,
            device: 0,
            target_gpt_id: plan.target.disk.identity.clone(),
            target_serial: "f9cbe1570c7f4ceeadb2".into(),
            target_capacity: plan.target.disk.capacity.unwrap(),
            initial_layout: "empty-gpt".into(),
            source_gpt_id: "gpt:130bd31e-c4d2-4ebb-84be-047120c5911a".into(),
            backup_gpt_id: "gpt:d837ddc5-2657-47cf-bfb2-b871b112d149".into(),
            evidence_gpt_id: "gpt:f45c9065-179b-4a09-be28-a96a02ba30ce".into(),
            backup_operation: "ar-abcd-ef-01".into(),
            backup_manifest_sha256: "0".repeat(64),
        };

        (plan, registration)
    }

    #[test]
    fn signature_binds_every_host_claim() {
        let (_, registration) = fixture();
        let key = SigningKey::from_bytes(&[17; 32]);
        let encode = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        let public_key = encode(key.verifying_key().as_bytes());
        let mut payload = serde_json::to_vec(&registration).unwrap();
        let signature = encode(&key.sign(&payload).to_bytes());

        let verified = authenticate(&payload, &signature, &public_key).unwrap();
        assert_eq!(verified.target_gpt_id, registration.target_gpt_id);

        payload.push(b' ');
        assert!(authenticate(&payload, &signature, &public_key).is_err());
        assert!(authenticate(&payload, &signature, &"00".repeat(32)).is_err());
    }

    #[test]
    fn signed_registration_still_requires_review_and_freshness() {
        let (plan, registration) = fixture();
        let token = registration.confirmation();

        assert!(registration.validate(&plan, &token, 150).is_ok());
        assert!(registration.validate(&plan, &token, 99).is_err());
        assert!(registration.validate(&plan, &token, 200).is_err());
        assert!(registration
            .validate(&plan, "other operation", 150)
            .is_err());

        let mut modified = plan;
        modified.media.sha256 = "0".repeat(64);
        assert!(registration.validate(&modified, &token, 150).is_err());
    }

    #[test]
    fn changed_layout_capacity_or_write_protection_fails_closed() {
        let (plan, registration) = fixture();
        let original = plan.target.disk;

        let mut disk = original.clone();
        disk.read_only = Some(true);
        assert!(registration.verify_empty_disk(&disk).is_err());

        let mut disk = original.clone();
        disk.capacity = Some(40 * 1024 * MIB);
        assert!(registration.verify_empty_disk(&disk).is_err());

        let mut disk = original.clone();
        disk.style = Some("MBR".into());
        assert!(registration.verify_empty_disk(&disk).is_err());

        let mut disk = original.clone();
        disk.partitions.push(archrescue_storage::Partition {
            number: 1,
            id: None,
            offset: MIB,
            length: MIB,
            kind: "test".into(),
            efi: false,
        });
        assert!(registration.verify_empty_disk(&disk).is_err());

        let mut disk = original;
        disk.identity = registration.source_gpt_id.clone();
        assert!(registration.verify_empty_disk(&disk).is_err());
    }

    #[test]
    fn registration_rejects_protected_disk_overlap() {
        let (plan, mut registration) = fixture();
        registration.backup_gpt_id = registration.target_gpt_id.clone();
        let token = registration.confirmation();

        assert!(registration.validate(&plan, &token, 150).is_err());
    }
}
