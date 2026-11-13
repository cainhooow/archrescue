#!/usr/bin/env python3
"""Issue a short-lived deployment registration from independent VirtualBox and guest evidence."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import stat
import xml.etree.ElementTree as ET
import struct
import subprocess
import time
import uuid
import zlib

CAPACITY = 80 * 1024**3
SECTOR = 512
MIB = 1024**2
TARGET_PORT = 2
SATA_SERIAL_KEY = f"VBoxInternal/Devices/ahci/0/Config/Port{TARGET_PORT}/SerialNumber"


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def vbox(*arguments):
    return subprocess.check_output(["VBoxManage", *map(str, arguments)], text=True)


def machine_info(vm):
    output = vbox("showvminfo", vm, "--machinereadable")
    return {
        key.strip('"'): value.strip('"')
        for line in output.splitlines()
        if "=" in line
        for key, value in [line.split("=", 1)]
    }


def owned_file(path, directory):
    path = Path(path)
    require(path.resolve(strict=True) == path and path.is_relative_to(directory),
            f"Path outside the registered lab run: {path}")
    info = path.stat()
    require(stat.S_ISREG(info.st_mode) and info.st_uid == os.getuid() and info.st_nlink == 1,
            f"Expected an independently owned regular lab file: {path}")
    return path


def registered_run(directory):
    require(directory.resolve(strict=True) == directory, "Use the absolute scratch run path")
    manifest_path = directory.parent / "manifests" / f"{directory.name}.json"
    manifest = json.loads(manifest_path.read_bytes())
    info = directory.stat()
    require(manifest["state"] == "active" and manifest["scratch"] == str(directory)
            and manifest["owner_uid"] == info.st_uid == os.getuid()
            and (manifest["device"], manifest["inode"]) == (info.st_dev, info.st_ino)
            and not info.st_mode & 0o077, "Expected a registered private scratch run")
    return manifest


def medium_uuid(info):
    return info["UUID"].lower()


def medium_info(path):
    return {
        key.strip(): value.strip()
        for line in vbox("showmediuminfo", "disk", path).splitlines()
        if ":" in line
        for key, value in [line.split(":", 1)]
    }


def inspect_vdi(path, capacity, require_empty=False):
    with path.open("rb") as stream:
        header = stream.read(440)
        signature, version = struct.unpack_from("<II", header, 64)
        header_bytes, variant, flags = struct.unpack_from("<III", header, 72)
        blocks_offset, data_offset = struct.unpack_from("<II", header, 340)
        disk_bytes, block_size, overhead, blocks = struct.unpack_from("<QIII", header, 368)
        require((signature, version, header_bytes, variant, flags) ==
                (0xBEDA107F, 0x00010001, 400, 1, 0)
                and disk_bytes == capacity and block_size == MIB and overhead == 0
                and blocks * MIB == capacity and blocks_offset >= 472
                and data_offset >= blocks_offset + blocks * 4,
                "Expected a standard dynamic base VDI")
        disk_uuid = str(uuid.UUID(bytes_le=header[392:408]))
        stream.seek(blocks_offset)
        mapping = struct.unpack(f"<{blocks}I", stream.read(blocks * 4))
        file_length = path.stat().st_size
        allocated = [entry for entry in mapping if entry < 0xFFFFFFFE]
        require(len(allocated) == len(set(allocated))
                and all(data_offset + (entry + 1) * MIB <= file_length for entry in allocated),
                "Invalid VDI allocation map")

        def read_sectors(lba, count):
            offset = lba * SECTOR
            length = count * SECTOR
            block, inner = divmod(offset, MIB)
            require(offset >= 0 and offset + length <= capacity and inner + length <= MIB,
                    "Unexpected GPT metadata bounds")
            mapped = mapping[block]
            if mapped >= 0xFFFFFFFE:
                return bytes(length)
            stream.seek(data_offset + mapped * MIB + inner)
            return stream.read(length)

        if require_empty:
            require(all(index in (0, blocks - 1) for index, item in enumerate(mapping)
                        if item < 0xFFFFFFFE), "Target contains allocated non-GPT blocks")
            mbr = read_sectors(0, 1)
            require(mbr[510:] == b"\x55\xaa" and mbr[450] == 0xEE
                    and not any(mbr[462:510]), "Target lacks a protective MBR")

        last = capacity // SECTOR - 1
        disk_ids = []
        for lba, other in [(1, last), (last, 1)]:
            sector = read_sectors(lba, 1)
            size, crc = struct.unpack_from("<II", sector, 12)
            require(sector[:8] == b"EFI PART" and 92 <= size <= SECTOR
                    and zlib.crc32(sector[:16] + bytes(4) + sector[20:size]) == crc
                    and struct.unpack_from("<QQ", sector, 24) == (lba, other),
                    "Invalid GPT header")
            entries_lba, count, entry_size, entries_crc = struct.unpack_from("<QIII", sector, 72)
            require(count == 128 and entry_size == 128 and
                    entries_lba == (2 if lba == 1 else last - 32), "Unexpected GPT entry table")
            entries = read_sectors(entries_lba, 32)
            require(zlib.crc32(entries) == entries_crc and (not require_empty or not any(entries)),
                    "GPT table checksum failed or the target is populated")
            disk_ids.append(str(uuid.UUID(bytes_le=sector[56:72])))

        require(disk_ids[0] == disk_ids[1] and uuid.UUID(disk_ids[0]).int, "GPT identity mismatch")
        if require_empty:
            primary = read_sectors(0, MIB // SECTOR)
            secondary = read_sectors(last + 1 - MIB // SECTOR, MIB // SECTOR)
            require(not any(primary[34 * SECTOR:]) and not any(secondary[:-33 * SECTOR]),
                    "Unexpected target data outside the GPT structures")

    return disk_uuid, f"gpt:{disk_ids[0]}"


def attest(args):
    run = args.run
    manifest = registered_run(run)
    info = machine_info(args.vm)
    machine_file = owned_file(info["CfgFile"], run)
    require(info["UUID"] == info["hardwareuuid"] == args.vm
            and info["VMState"] == "poweroff" and info["firmware"] == "EFI"
            and info["name"] == f"ArchRescue-Wave5-{run.name[:8]}"
            and info.get("CurrentSnapshotUUID") is None
            and info.get("nic1") == "none", "Expected the powered-off disposable UEFI guest")
    tree = ET.parse(machine_file)
    namespace = {"v": "http://www.virtualbox.org/"}
    require(not tree.findall(".//v:Snapshot", namespace)
            and not tree.findall(".//v:SharedFolder", namespace),
            "The disposable guest cannot have snapshots or shared folders")
    hard_disk_slots = {
        (controller.attrib["name"], int(device.attrib["port"]), int(device.attrib["device"]))
        for controller in tree.findall(".//v:StorageController", namespace)
        for device in controller.findall("v:AttachedDevice", namespace)
        if device.attrib["type"] == "HardDisk"
    }
    require(hard_disk_slots == {("SATA", port, 0) for port in range(2, 6)},
            "Unexpected hard disk attached to the disposable guest")
    require(info["storagecontrollername0"] == "SATA"
            and info["storagecontrollertype0"] == "IntelAhci", "Unexpected SATA controller")
    for port, dependency in enumerate(manifest["dependencies"]):
        require(port < 2 and info[f"SATA-{port}-0"] == dependency, "Unexpected optical attachment")
    require(len(manifest["dependencies"]) == 2, "Register the two optical dependencies")

    disks = {}
    for port, role in enumerate(("target", "source", "backup", "evidence"), TARGET_PORT):
        path = owned_file(run / f"{role}.vdi", run)
        require(info[f"SATA-{port}-0"] == str(path), f"Unexpected {role} attachment")
        capacity = CAPACITY if role == "target" else 768 * MIB
        vdi_uuid, gpt_id = inspect_vdi(path, capacity, require_empty=role == "target")
        medium = medium_info(path)
        require(medium_uuid(medium) == info[f"SATA-ImageUUID-{port}-0"] == vdi_uuid
                and medium["Parent UUID"] == "base" and medium["Location"] == str(path)
                and medium["Storage format"] == "VDI" and medium["Encryption"] == "disabled"
                and medium["Capacity"] == f"{capacity // MIB} MBytes"
                and re.fullmatch(r"normal \(base\)", medium["Type"])
                and medium.get("In use by VMs") == f'{info["name"]} (UUID: {args.vm})'
                and "Child UUIDs" not in medium, f"Unexpected VirtualBox identity for {role}")
        disks[role] = (vdi_uuid, gpt_id)

    require(len({identity for _, identity in disks.values()}) == len(disks),
            "Duplicated GPT disk identities")
    serial = disks["target"][0].replace("-", "")[:20]
    require(vbox("getextradata", args.vm, SATA_SERIAL_KEY).strip() == f"Value: {serial}",
            "Target SATA serial is not pinned in the VM configuration")

    plan = json.loads(args.plan.read_bytes())
    observed = json.loads(args.observation.read_bytes())
    backup = json.loads(args.backup_manifest.read_bytes())["data"]
    target = observed["target"]
    require(observed["vm_uuid"] == args.vm and observed["attachment"]["serial"].lower() == serial
            and observed["attachment"]["product"].strip().upper() == "VBOX HARDDISK"
            and observed["attachment"]["bus"] in (3, 11)
            and observed["attachment"]["logical_sector"] == 512
            and observed["attachment"]["physical_sector"] == 512
            and target["identity"] == disks["target"][1] and target["capacity"] == CAPACITY
            and target["style"] == "GPT" and target["read_only"] is False
            and not target["partitions"] and not target["errors"]
            and plan["target"]["disk"] == target and plan["target"]["firmware"] == "UEFI"
            and plan["result"] == "Supported" and plan["intent"] == "Clean"
            and plan["media"]["image_type"] == "Wim" and plan.get("lab_layout")
            and re.fullmatch("[0-9a-f]{64}", plan["fingerprint"]),
            "Guest observation and independently inspected target or plan differ")
    require(backup["source_identity"]["filesystem_id"].lower().startswith(disks["source"][1] + ";")
            and backup["files"] and re.fullmatch("[0-9a-f]{64}", backup["plan_fingerprint"]),
            "Backup manifest does not identify the protected lab source")
    now = int(time.time())
    registration = {
        "schema_version": 1,
        "purpose": "archrescue-disposable-uefi-v1",
        "operation_id": str(uuid.uuid4()),
        "issued_unix": now,
        "expires_unix": now + 3600,
        "plan_fingerprint": plan["fingerprint"],
        "vm_uuid": args.vm,
        "target_vdi_uuid": disks["target"][0],
        "target_backing_file": str(run / "target.vdi"),
        "controller": "SATA",
        "port": TARGET_PORT,
        "device": 0,
        "target_gpt_id": disks["target"][1],
        "target_serial": serial,
        "target_capacity": CAPACITY,
        "initial_layout": "empty-gpt",
        "source_gpt_id": disks["source"][1],
        "backup_gpt_id": disks["backup"][1],
        "evidence_gpt_id": disks["evidence"][1],
        "backup_operation": backup["operation_id"],
        "backup_manifest_sha256": backup["plan_fingerprint"],
    }

    private_key = owned_file(args.key, run)
    require(not private_key.stat().st_mode & 0o077, "The signing key must be host-private")
    args.output.mkdir(mode=0o700)
    path = args.output / "registration.json"
    path.write_text(json.dumps(registration, indent=2) + "\n", encoding="utf-8")
    signature = subprocess.check_output(
        ["openssl", "pkeyutl", "-sign", "-rawin", "-inkey", private_key, "-in", path])
    require(len(signature) == 64, "Expected an Ed25519 signature")
    (args.output / "registration.sig").write_text(signature.hex() + "\n", encoding="ascii")
    (args.output / "confirmation.txt").write_text(
        f'AUTORIZAR-LAB:{registration["operation_id"]}:{plan["fingerprint"]}\n', encoding="ascii")
    with (run / "target.vdi").open("rb") as target_file:
        target_sha256 = hashlib.file_digest(target_file, "sha256").hexdigest()
    (args.output / "host-attestation.json").write_text(json.dumps({
        "machine_configuration_sha256": hashlib.sha256(machine_file.read_bytes()).hexdigest(),
        "target_vdi_sha256": target_sha256,
        "disk_identities": disks,
        "vm": info,
    }, indent=2) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path)
    parser.add_argument("vm")
    parser.add_argument("plan", type=Path)
    parser.add_argument("observation", type=Path)
    parser.add_argument("backup_manifest", type=Path)
    parser.add_argument("key", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    try:
        attest(args)
    except (ValueError, OSError, KeyError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Registration refused: {error}\n")


if __name__ == "__main__":
    main()
