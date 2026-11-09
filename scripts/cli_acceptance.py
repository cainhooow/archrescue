#!/usr/bin/env python3
"""Independently verify the executable using controlled files on two real filesystems."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[1]
BINARY = REPO / 'target/release/archrescue-boot-shell'
BASE = REPO / '.agent-tmp/cli'
BASE.mkdir(parents=True, exist_ok=True)
external = os.environ.get('ARCHRESCUE_TEST_DESTINATION')
if not external:
    raise SystemExit('BLOCKED: ARCHRESCUE_TEST_DESTINATION is required')
source = Path(tempfile.mkdtemp(prefix='source-', dir=BASE))
destination = Path(tempfile.mkdtemp(prefix='destination-', dir=external))
output = REPO / '.agent-tmp/cli-evidence'
output.mkdir(exist_ok=True)

def run(*args, expected=0):
    result = subprocess.run([str(BINARY), *map(str, args)], capture_output=True, text=True)
    assert result.returncode == expected, (args, result.returncode, result.stderr)
    (output / f'{args[0]}-stdout.txt').write_text(result.stdout)
    (output / f'{args[0]}-stderr.txt').write_text(result.stderr)
    return result.stdout

def manifest(root):
    return {str(p.relative_to(root)): {'bytes': p.stat().st_size, 'sha256': hashlib.sha256(p.read_bytes()).hexdigest()}
            for p in sorted(root.rglob('*')) if p.is_file()}

try:
    assert source.stat().st_dev != destination.stat().st_dev
    (source / 'perfil/Documentos').mkdir(parents=True)
    (source / 'perfil/vazia').mkdir()
    (source / 'perfil/Documentos/dados.bin').write_bytes(bytes(range(256)) * 1024)
    (source / 'perfil/olá.txt').write_text('Dados de teste — ArchRescue\n', encoding='utf-8')
    (source / 'perfil/zero.txt').write_bytes(b'')
    before = manifest(source)
    version = run('--version').strip()
    expected_build = os.environ.get('ARCHRESCUE_EXPECTED_BUILD') or subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=REPO, text=True).strip()
    assert f'build={expected_build}' in version, 'Stale or unidentified binary; rebuild successfully before acceptance'

    browse = json.loads(run('browse', source / 'perfil'))
    assert len(browse) == 4
    assert json.loads(run('discover', source))['installations'] == []
    result = json.loads(run('backup', source, destination, 'perfil'))
    assert result['operation']['state'] == 'COMPLETED'
    assert len(result['files']) == 3 and all(f['verified'] for f in result['files'])
    assert result['copied_bytes'] == result['planned_bytes'] == sum(f['bytes'] for f in before.values())
    assert manifest(source) == before == manifest(destination)
    assert (destination / 'perfil/vazia').is_dir()
    # Restore comparison uses an independent copier into another controlled directory.
    restore = BASE / ('restore-' + source.name)
    shutil.copytree(destination / 'perfil', restore / 'perfil')
    assert manifest(restore) == before
    shutil.rmtree(restore)
    completed = result
    # Existing targets are rejected; successful output was saved separately first.
    (output / 'backup-completed.json').write_text(json.dumps(completed, indent=2))
    conflict = json.loads(run('backup', source, destination, 'perfil', expected=1))
    assert conflict['operation']['state'] == 'FAILED'
    assert conflict['operation']['errors'][0]['kind'] == 'CONFLICT'
    assert manifest(source) == before
    report_destination = destination / 'reports'
    report_destination.mkdir()
    run('report', report_destination, source)
    reports = list(report_destination.glob('*/report.json'))
    assert len(reports) == 1
    report = json.loads(reports[0].read_text())
    assert report['version'] == '0.0.1' and report['runtime_os'] == 'linux'
    assert reports[0].with_name('report.txt').is_file()
    assert manifest(source) == before
    summary = {
        'status': 'PASS', 'scope': 'Linux CLI fixtures only; not Windows or VM acceptance',
        'version_output': version, 'executable_sha256': hashlib.sha256(BINARY.read_bytes()).hexdigest(),
        'source_identity': f'unix-dev:{source.stat().st_dev}',
        'destination_identity': f'unix-dev:{destination.stat().st_dev}',
        'file_count': len(before), 'byte_count': sum(f['bytes'] for f in before.values()),
        'source_before_equals_after': True, 'destination_equals_source': True,
        'restored_copy_equals_source': True, 'independent_hashes': before,
        'operation_id': completed['operation']['id'], 'conflict_status': conflict['operation']['state'],
        'report_roundtrip': 'PASS', 'fixture_cleanup': 'owned directories removed after assertions'
    }
    (output / 'summary.json').write_text(json.dumps(summary, indent=2, ensure_ascii=False))
    print(json.dumps(summary, indent=2, ensure_ascii=False))
finally:
    shutil.rmtree(source)
    shutil.rmtree(destination)
