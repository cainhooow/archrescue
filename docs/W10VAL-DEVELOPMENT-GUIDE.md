# W10Val development guide

The corrective run reused the installed W10Val toolchain and transferred a verified bundle from the local corrective branch. It records the additional live checkpoint and the elevated interactive task used to run the ISO build. [Corrective homologation](qa/P0-CORRECTIVE-HOMOLOGATION.md)

Use W10Val for native Windows builds. Use a separate network-disconnected UEFI VM for boot acceptance. The [baseline](evidence/windows/W10VAL-BASELINE.md) records the guest configuration and [provisioning](evidence/windows/W10VAL-PROVISIONING.md) records the recovery snapshot and installed components.

## Transfer the source revision

From the Linux checkout, inspect the selected branch and retain any uncommitted work on the host. Create a bundle from the committed ref and transfer it using the existing loopback SSH alias.

```bash
git status --short --branch
git rev-parse refs/heads/main
mkdir -p .agent-tmp
git bundle create .agent-tmp/archrescue-main.bundle refs/heads/main
git bundle verify .agent-tmp/archrescue-main.bundle
sha256sum .agent-tmp/archrescue-main.bundle
scp -o BatchMode=yes .agent-tmp/archrescue-main.bundle w10val:
```

In a fresh Windows working directory, compare the bundle hash, clone it, and verify the expected full commit ID and empty porcelain status. Keep logs, installers and test evidence outside the tracked tree.

```powershell
Get-FileHash -Algorithm SHA256 "$env:USERPROFILE\archrescue-main.bundle"
git -c core.autocrlf=false clone --branch main "$env:USERPROFILE\archrescue-main.bundle" source
Set-Location source
git rev-parse HEAD
git status --porcelain
Get-FileHash Cargo.lock -Algorithm SHA256
```

## Initialize the build shell

From a Windows `cmd.exe` session, discover the C++ Build Tools instance and use its x64 environment. Add Git and Rustup's proxy directory to this process PATH if SSH inherited an older PATH.

```bat
set "PATH=%USERPROFILE%\.cargo\bin;%ProgramFiles%\Git\cmd;%PATH%"
"%ProgramFiles(x86)%\Microsoft Visual Studio\Installer\vswhere.exe" -latest -products Microsoft.VisualStudio.Product.BuildTools -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
call "<reported installation path>\Common7\Tools\VsDevCmd.bat" -arch=x64 -host_arch=x64
where cl link dumpbin lib cargo rustc git
rustc -Vv
rustup show
rustup target list --installed
```

Use the recorded Rust release for reproducing a previous artifact. The stable channel may resolve to a newer release on subsequent installs. Preserve `Cargo.lock` and the target's static CRT flags.

## Build and test

Open PowerShell from the initialized x64 shell and enter the clean checkout. Bind the executable build identity to the committed source. Check the architecture, executable hash and imported DLLs.

```powershell
$env:ARCHRESCUE_BUILD_ID = git rev-parse HEAD
cargo fmt --all --check
cargo check --workspace --all-targets --locked --target x86_64-pc-windows-msvc
cargo clippy --workspace --all-targets --locked --target x86_64-pc-windows-msvc -- -D warnings
cargo build --release --locked --target x86_64-pc-windows-msvc -p archrescue-boot-shell
$exe = 'target/x86_64-pc-windows-msvc/release/archrescue-boot-shell.exe'
dumpbin /headers $exe
dumpbin /dependents $exe
Get-FileHash -Algorithm SHA256 $exe
& $exe --version
```

For the native suite, confirm the destination directory is on a disposable NTFS disk with a different VirtualBox medium UUID and guest disk number than the checkout. Inspect the serial, attachment port and volume label first.

```powershell
Get-Disk | Format-Table Number,SerialNumber,PartitionStyle,IsBoot,IsSystem
Get-Partition | Format-Table DiskNumber,PartitionNumber,DriveLetter,Size
Get-Volume | Format-Table DriveLetter,FileSystem,FileSystemLabel,UniqueId
$env:ARCHRESCUE_TEST_DESTINATION = '<verified disposable destination directory>'
cargo test --workspace --locked --target x86_64-pc-windows-msvc
```

Run the no-argument EXE in the guest's interactive session for GUI inspection. Check that the observed PID belongs to that session and hash the executable at its image path.

## Build local WinPE media

Use the matching ADK and WinPE add-on in an elevated x64 developer shell. Load `Deployment Tools\DandISetEnv.bat` under the ADK installation, put `Deployment Tools\amd64\DISM` first on the process PATH, then return to the clean checkout.

```powershell
Get-Command cargo,rustc,git,copype.cmd,MakeWinPEMedia.cmd,dism.exe,dumpbin.exe
dism.exe /English /Get-MountedImageInfo
git status --porcelain
.\packaging\winpe\build.ps1
dism.exe /English /Get-MountedImageInfo
```

Run when the dedicated build host has no pending WIM mount or servicing job. Keep the script transcript, build metadata, PE import list and checksums together. The procedure produces an ISO for local use. See the [WinPE build evidence](evidence/windows/WINPE-ISO-BUILD.md) and the [VM acceptance procedure](../tests/vm/ACCEPTANCE.md).

## Recover the build VM

See the [provisioning record](evidence/windows/W10VAL-PROVISIONING.md) for the snapshot UUID. Snapshot restore discards newer guest disk state. Save needed build output and logs first, gracefully shut down W10Val, confirm the snapshot identity, and restore only when recovering the pre-toolchain state. Start W10Val and recheck the existing batch-mode SSH connection.

## BCD diagnostic workspace

BCD enumeration works from a temporary copy because BCDEdit can update the opened hive during recovery. The process temp path must be on a verified separate disk from the BCD source. A shared disk or an unresolved extent produces signature-only evidence. Review the diagnostic text before treating a store as enumerated.
