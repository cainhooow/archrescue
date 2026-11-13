import hashlib, json, os, subprocess, sys
from pathlib import Path, PureWindowsPath

run, output=map(Path,sys.argv[1:3])
env=dict(os.environ,LD_LIBRARY_PATH=str(run/'ntfs/usr/lib'))
def tool(name,*args):
    return subprocess.check_output([run/'ntfs/usr/bin'/name,*args], env=env)
def files(disk,path):
    return tool('ntfsls','-p',path,disk).decode().splitlines()
def read(disk,path):
    return tool('ntfscat',disk,path)
def sha(data):
    return hashlib.sha256(data).hexdigest()
source,dest=[run/f'part-{i}.ntfs' for i in (1,2)]
operation,=[name for name in files(dest,'/ArchRescue') if name.startswith('archrescue-backup-')]
base='/ArchRescue/'+operation
manifest_data=read(dest,base+'/manifest.json')
status_data=read(dest,base+'/status.json')
manifest=json.loads(manifest_data)['data']
status=json.loads(status_data)['data']
assert status['state']=='COMPLETED',status
assert status['plan_fingerprint']==manifest['plan_fingerprint']
checks=[]
for record in manifest['files']:
    path='/'+PureWindowsPath(record['relative']).as_posix()
    payload=read(dest,base+'/payload'+path)
    assert len(payload)==record['bytes'] and sha(payload)==record['sha256']
    assert read(source,path)==payload
    checks.append({'path':path,'bytes':len(payload),'sha256':sha(payload)})
assert checks,manifest
(output/'backup-manifest.json').write_bytes(manifest_data)
(output/'backup-status.json').write_bytes(status_data)
hashes={}
for name in files(dest,'/'):
    if name.startswith('archrescue-report-'):
        for filename in ('report.json','report.txt'):
            data=read(dest,f'/{name}/{filename}')
            (output/filename).write_bytes(data)
            hashes[filename]=sha(data)
    elif name.startswith('archrescue-restore-handoff-'):
        data=read(dest,'/'+name)
        assert json.loads(data)['manifest_sha256']==manifest['plan_fingerprint']
        (output/'restore-handoff.json').write_bytes(data)
        hashes['restore-handoff.json']=sha(data)
logs=[]
for name in sorted(files(dest,'/ar-test')):
    if name.endswith(('.LOG','.EXIT')):
        data=read(dest,'/ar-test/'+name)
        logs.append(f'===== {name} =====\n'+data.decode(errors='replace'))
(output/'windows-winpe-tests.log').write_text('\n'.join(logs))
(output/'independent-readback.json').write_text(json.dumps({'payload':checks,'exports':hashes,'operation':operation,'manifest_sha256':sha(manifest_data)},indent=2)+'\n')
