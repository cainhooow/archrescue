#!/usr/bin/env python3
"""Manage explicitly owned ArchRescue scratch runs on the Linux build/QA host."""

import argparse
import fcntl
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import uuid


GIB = 1024 ** 3
REPO = Path(__file__).resolve().parents[1]


def scratch_root():
    cache = Path(os.environ.get("XDG_CACHE_HOME", Path.home() / ".cache"))
    root = Path(os.environ.get("ARCHRESCUE_SCRATCH_ROOT", cache / "archrescue"))
    if not root.is_absolute() or root.resolve() != root or root.is_relative_to(REPO):
        raise ValueError("ArchRescue scratch must be an absolute, non-link path outside the repository")
    root.mkdir(mode=0o700, parents=True, exist_ok=True)
    if root.stat().st_uid != os.getuid() or root.stat().st_mode & 0o022:
        raise ValueError("ArchRescue scratch must be owned by the operator and not group/world writable")
    return root


def check_space(root, budget, reserve):
    available = shutil.disk_usage(root).free
    if budget <= 0 or reserve < 0:
        raise ValueError("A positive budget and nonnegative reserve are required")
    if available < budget + reserve:
        raise ValueError(f"{root}: {available} bytes free, {budget + reserve} required")
    scratch = REPO / ".agent-tmp"
    if scratch.is_dir():
        accounted = subprocess.run(
            ["du", "-sB1", str(scratch)], check=True, capture_output=True, text=True,
        )
        size = int(accounted.stdout.split()[0])
        if size > 2 * GIB:
            print(f"WARNING: repository scratch accounts for {size} allocated bytes", file=os.sys.stderr)
    return available


def write_manifest(root, manifest):
    path = root / (manifest["id"] + ".json")
    temporary = path.with_suffix(f".{uuid.uuid4().hex}.new")
    with temporary.open("x", encoding="utf-8") as stream:
        json.dump(manifest, stream, indent=2)
        stream.flush()
        os.fsync(stream.fileno())
    temporary.replace(path)


def create(root, args):
    available = check_space(root, args.budget_bytes, args.reserve_bytes)
    manifest_directory = root / "manifests"
    manifest_directory.mkdir(exist_ok=True)
    if manifest_directory.is_symlink():
        raise ValueError("Manifest directory is a symbolic link")
    for path in manifest_directory.glob("*.json"):
        if path.is_symlink():
            raise ValueError(f"Manifest is a symbolic link: {path}")
        current = json.loads(path.read_text(encoding="utf-8"))
        if current["state"] in ("active", "cleaning"):
            raise ValueError(f"An earlier run still owns its scratch: {path}")

    run_id = uuid.uuid4().hex
    directory = root / run_id
    directory.mkdir(mode=0o700)
    metadata = directory.stat()
    manifest = {
        "id": run_id,
        "owner_uid": os.getuid(),
        "purpose": args.purpose,
        "expected_lifespan": "Through build or QA and evidence promotion",
        "scratch": str(directory),
        "device": metadata.st_dev,
        "inode": metadata.st_ino,
        "budget_bytes": args.budget_bytes,
        "free_at_start_bytes": available,
        "dependencies": args.dependency,
        "retain": args.retain,
        "state": "active",
    }
    write_manifest(manifest_directory, manifest)
    print(directory)


def verify_inactive(directory, paths):
    listing = subprocess.run(
        ["lsof", "+D", str(directory)], capture_output=True, text=True,
    )
    if listing.returncode != 1 or listing.stdout.strip() or listing.stderr.strip():
        raise ValueError(f"Open-file check could not establish that {directory} is inactive")

    disk_files = {path for path in paths if path.suffix.lower() in {".vdi", ".vhd", ".vhdx", ".iso", ".qcow2", ".raw", ".vmdk"}}
    if disk_files and shutil.which("VBoxManage"):
        registered = "".join(
            subprocess.check_output(["VBoxManage", "list", kind], text=True)
            for kind in ("hdds", "dvds")
        )
        if any(str(path) in registered for path in disk_files):
            raise ValueError("Scratch contains a registered VirtualBox medium")


def close(root, args):
    run_id = uuid.UUID(args.run_id).hex
    manifests = root / "manifests"
    path = manifests / f"{run_id}.json"
    if path.is_symlink() or manifests.is_symlink():
        raise ValueError("Manifest path traverses a symbolic link")
    manifest = json.loads(path.read_text(encoding="utf-8"))
    if manifest["state"] == "cleaned":
        return

    directory = root / run_id
    if manifest["scratch"] != str(directory) or manifest["owner_uid"] != os.getuid():
        raise ValueError("Scratch path identity changed")
    try:
        metadata = directory.lstat()
    except FileNotFoundError:
        if manifest["state"] != "cleaning" or not args.cleanup:
            raise
        manifest.update(state="cleaned", allocated_bytes=0)
        write_manifest(manifests, manifest)
        return

    if (metadata.st_dev, metadata.st_ino) != (manifest["device"], manifest["inode"]) or not stat.S_ISDIR(metadata.st_mode):
        raise ValueError("Scratch path identity changed")

    paths = list(directory.rglob("*"))
    for item in paths:
        info = item.lstat()
        if (
            stat.S_ISLNK(info.st_mode)
            or info.st_dev != metadata.st_dev
            or info.st_uid != metadata.st_uid
            or (not stat.S_ISREG(info.st_mode) and not stat.S_ISDIR(info.st_mode))
        ):
            raise ValueError(f"Unexpected scratch entry: {item}")

    size = sum(item.lstat().st_blocks * 512 for item in paths) + metadata.st_blocks * 512
    if size > manifest["budget_bytes"]:
        print(f"WARNING: run consumed {size} bytes against budget {manifest['budget_bytes']}", file=os.sys.stderr)
    if not args.cleanup:
        manifest.update(state="retained", allocated_bytes=size)
        write_manifest(manifests, manifest)
        return

    for output in manifest["retain"]:
        retained = Path(output)
        if not retained.is_absolute() or retained.is_relative_to(directory) or not retained.exists():
            raise ValueError(f"Required evidence has not been promoted: {output}")
    verify_inactive(directory, paths)
    if manifest["state"] != "cleaning":
        manifest.update(
            state="cleaning",
            created_paths=[str(item.relative_to(directory)) for item in paths],
            allocated_before_cleanup=size,
        )
        write_manifest(manifests, manifest)
    shutil.rmtree(directory)
    manifest.update(state="cleaned", allocated_bytes=0)
    write_manifest(manifests, manifest)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    for name in ("check", "open"):
        command = commands.add_parser(name)
        command.add_argument("--budget-bytes", type=int, required=True)
        command.add_argument("--reserve-bytes", type=int, default=5 * GIB)
        if name == "open":
            command.add_argument("purpose")
            command.add_argument("--dependency", action="append", default=[])
            command.add_argument("--retain", action="append", default=[])
    command = commands.add_parser("close")
    command.add_argument("run_id")
    command.add_argument("--cleanup", action="store_true")
    args = parser.parse_args()

    try:
        root = scratch_root()
        descriptor = os.open(root / ".lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "a") as lock:
            fcntl.flock(lock, fcntl.LOCK_EX)
            if args.command == "check":
                check_space(root, args.budget_bytes, args.reserve_bytes)
            elif args.command == "open":
                create(root, args)
            else:
                close(root, args)
    except (OSError, ValueError, subprocess.SubprocessError) as error:
        parser.exit(1, f"ArchRescue scratch: {error}\n")


if __name__ == "__main__":
    main()
