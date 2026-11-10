#!/usr/bin/env python3
"""Kill a real backup process and independently verify recovery on disposable filesystems."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time


def sha256(path):
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def invoke(executable, *arguments):
    result = subprocess.run(
        [executable, *map(str, arguments)], check=True, capture_output=True, text=True,
        encoding="utf-8",
    )
    return json.loads(result.stdout)


def demonstrate(executable, source, destination, evidence):
    files = ("01-pronto.txt", "02-recuperação.bin", "03-vazio")
    (source / files[0]).write_bytes(b"published before interruption")
    with (source / files[1]).open("wb") as file:
        data = os.urandom(1024 * 1024)
        for _ in range(256):
            file.write(data)
    (source / files[2]).touch()
    expected = {name: sha256(source / name) for name in files}

    with (evidence / "interrupted.log").open("wb") as log:
        process = subprocess.Popen(
            [executable, "backup-start", source, destination, *files],
            stdout=log, stderr=subprocess.STDOUT,
        )
        try:
            deadline = time.monotonic() + 120
            while time.monotonic() < deadline and process.poll() is None:
                operations = list(destination.glob("archrescue-backup-*"))
                if len(operations) == 1:
                    directory = operations[0]
                    partial = directory / "journal" / "1.part"
                    try:
                        partial_bytes = partial.stat().st_size
                    except FileNotFoundError:
                        partial_bytes = 0
                    if 0 < partial_bytes < (source / files[1]).stat().st_size:
                        assert (directory / "payload" / files[0]).exists()
                        process.kill()
                        break
                time.sleep(0.002)
            else:
                raise RuntimeError("Could not interrupt during a real write")
        finally:
            if process.poll() is None:
                process.kill()
            interrupted_exit = process.wait()

    assert interrupted_exit != 0
    assert (directory / "journal" / "1.part").exists()
    assert not (directory / "payload" / files[1]).exists()
    assert sha256(directory / "payload" / files[0]) == expected[files[0]]
    published_before = (directory / "payload" / files[0]).stat()
    discovered, = invoke(executable, "backup-list", destination)
    assert discovered["status"]["state"] == "RUNNING"
    assert discovered["error"] is None

    resumed = invoke(executable, "backup-resume", source, directory)
    assert resumed["status"]["state"] == "COMPLETED"
    assert resumed["status"]["resumed"] and resumed["progress"]["skipped_files"] == 1
    assert resumed["progress"]["verified_files"] == len(files)
    assert (directory / "payload" / files[0]).stat().st_ino == published_before.st_ino
    assert not (directory / "journal" / "1.part").exists()
    assert invoke(executable, "backup-verify", source, directory)["status"]["state"] == "COMPLETED"

    restored = destination / "restored"
    shutil.copytree(directory / "payload", restored)
    assert {name: sha256(restored / name) for name in files} == expected
    assert {name: sha256(directory / "payload" / name) for name in files} == expected
    assert {name: sha256(source / name) for name in files} == expected

    trace = {
        "source": str(source),
        "destination": str(destination),
        "interruption": {"exit": interrupted_exit, "observed_partial_bytes": partial_bytes},
        "discovery": discovered,
        "resumed": resumed,
        "independent_source_payload_and_restore_sha256": expected,
    }
    (evidence / "resume-trace.json").write_text(json.dumps(trace, indent=2), encoding="utf-8")
    print("PASS. Killed during copy, discovered, resumed, verified, restored and matched SHA-256.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("executable", type=Path)
    parser.add_argument("source_filesystem", type=Path)
    parser.add_argument("destination_filesystem", type=Path)
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    args.evidence.mkdir(parents=True, exist_ok=True)

    with (
        tempfile.TemporaryDirectory(dir=args.source_filesystem) as source,
        tempfile.TemporaryDirectory(dir=args.destination_filesystem) as destination,
    ):
        demonstrate(args.executable.resolve(), Path(source).resolve(), Path(destination).resolve(), args.evidence)


if __name__ == "__main__":
    main()
