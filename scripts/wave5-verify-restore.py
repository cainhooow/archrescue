#!/usr/bin/env python3
"""Independently compare Wave 5 source, backup and installed Windows NTFS readbacks."""

import argparse
import hashlib
import json
from pathlib import Path, PureWindowsPath
import subprocess


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def require(condition, message):
    if not condition:
        raise ValueError(message)


def verify(args):
    manifest = read_json(args.evidence / "manifest.json")["data"]
    deployment = read_json(args.evidence / "first-boot-handoff.json")
    boot = read_json(args.evidence / "installed-boot.json")
    profile = read_json(args.evidence / "installed-profile.json")
    plan = read_json(args.evidence / "restore-preview-renomear.json")
    report = read_json(args.evidence / "restore-result-renomear.json")
    abort = read_json(args.evidence / "restore-result-abortar.json")
    skip = read_json(args.evidence / "restore-preview-ignorar.json")
    recorded_conflict = read_json(args.evidence / "installed-conflict.json")
    source_hashes = read_json(args.evidence / "source-hashes.json")

    require(deployment["execution_state"] == "awaiting_first_boot" and not deployment["errors"],
            "Deployment record is incomplete")
    require(boot["firmware"] == 2 and boot["explorer_running"]
            and boot["disk_guid"].strip("{}").lower() == deployment["target_disk_identity"]["identity"][4:],
            "Installed Windows did not corroborate the deployment target")
    require(report["recovery"] == plan["recovery"] == deployment["restore"]
            and report["plan_sha256"] == plan["fingerprint"]
            and report["target_sid"] == plan["target_sid"] == profile["account_sid"] == boot["sid"]
            and report["backup_operation"] == manifest["operation_id"]
            and deployment["restore"]["backup_manifest_sha256"] == manifest["plan_fingerprint"]
            and not report["error"], "Recovery evidence cannot be joined across stages")
    require(abort["error"]["kind"] == "CONFLICT"
            and not any(file["published"] for file in abort["files"])
            and any(file["action"] == "Skip" for file in skip["files"]),
            "Conflict policy evidence is missing or inconsistent")

    def ntfs(tool, *arguments):
        return subprocess.check_output([args.tools / tool, *arguments])

    def read(disk, relative):
        return ntfs("ntfscat", disk, "/" + relative)

    def relative_path(path):
        windows_path = PureWindowsPath(path)
        return PureWindowsPath(*windows_path.parts[1:]).as_posix()

    source_root = "Users/Original"
    backup_root = f'ArchRescue/archrescue-backup-{manifest["operation_id"]}/payload'
    target_root = relative_path(profile["root"])
    by_source = {PureWindowsPath(item["source_relative"]).as_posix(): item for item in report["files"]}
    require(len(by_source) == len(report["files"]) == len(manifest["files"]) == len(source_hashes),
            "Source, manifest and restore file counts differ")
    files = []
    for entry in manifest["files"]:
        source = PureWindowsPath(entry["relative"]).as_posix()
        restored = by_source[source]
        require(restored["published"] and restored["outcome"] == "Verified",
                f"Restore publication missing for {source}")
        target = PureWindowsPath(restored["target_relative"]).as_posix()
        payloads = {
            "source": read(args.source, f"{source_root}/{source}"),
            "backup": read(args.backup, f"{backup_root}/{source}"),
            "restored": read(args.installed, f"{target_root}/{target}"),
        }
        hashes = {role: hashlib.sha256(data).hexdigest() for role, data in payloads.items()}
        require(set(hashes.values()) == {entry["sha256"]} and
                all(len(data) == entry["bytes"] for data in payloads.values())
                and source_hashes[source] == {"bytes": entry["bytes"], "sha256": entry["sha256"]},
                f"Independent SHA-256 or size check failed for {source}")
        files.append({"source": source, "destination": target, "bytes": entry["bytes"],
                      "sha256": hashes})

    empty = "Documents/empty-folder"
    for disk, root in [(args.source, source_root), (args.backup, backup_root),
                       (args.installed, target_root)]:
        children = ntfs("ntfsls", "-p", f"/{root}/{empty}", disk).decode().splitlines()
        require(not children, f"Expected empty directory on {disk}")

    desktop = relative_path(profile["folders"]["Desktop"])
    conflict = read(args.installed, f"{desktop}/conflict.txt")
    conflict_hash = hashlib.sha256(conflict).hexdigest()
    require(conflict == b"existing destination file"
            and conflict_hash == recorded_conflict["before"].lower() == recorded_conflict["after"].lower()
            and by_source["Desktop/conflict.txt"]["action"] == "Rename",
            "Existing destination file was not independently preserved")
    return {
        "recovery_operation": deployment["operation_id"],
        "deployment_plan_sha256": deployment["plan_fingerprint"],
        "backup_manifest_sha256": manifest["plan_fingerprint"],
        "restore_plan_sha256": plan["fingerprint"],
        "boot": boot,
        "file_count": len(files),
        "bytes": sum(file["bytes"] for file in files),
        "files": files,
        "empty_directory": empty,
        "preserved_conflict_sha256": conflict_hash,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for argument in ("evidence", "tools", "source", "backup", "installed", "output"):
        parser.add_argument(argument, type=Path)
    args = parser.parse_args()
    try:
        result = verify(args)
        with args.output.open("x", encoding="utf-8") as stream:
            json.dump(result, stream, indent=2, ensure_ascii=False)
            stream.write("\n")
    except (KeyError, OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"Independent restore verification failed: {error}\n")


if __name__ == "__main__":
    main()
