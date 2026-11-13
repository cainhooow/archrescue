# Wave 5 disposable Windows recovery

The executor is available only in x64 WinPE when the shell is compiled with the
`lab-deployment` feature and a per-run Ed25519 public key. Generate the key in the
registered host scratch directory and keep the private key there. Build the shell
with `ARCHRESCUE_LAB_PUBLIC_KEY` set to the 64 hexadecimal characters of the raw
public key. A normal build has no deployment execution entry point.

## Prepare and attest

1. Register a scratch run with `scripts/scratch.py open`. Register the historical
   WinPE ISO first and the mounted Windows client installation ISO second as
   dependencies. Create a new UEFI VirtualBox guest named
   `ArchRescue-Wave5-<first 8 characters of scratch ID>` with its settings stored
   inside that run. Disable networking and use an Intel AHCI controller named
   `SATA`. Attach the two optical dependencies to ports 0 and 1. Use a separate
   virtual optical controller to deliver the QA shell and scripts.
2. Prepare four new, dynamic base VDIs named `target.vdi`, `source.vdi`,
   `backup.vdi` and `evidence.vdi`, with no shared storage, snapshots or child
   dependencies. Attach them at SATA ports 2 through 5, device 0, in that order.
   The target is 80 GiB with matching primary and secondary empty GPT tables and
   no other allocated data. Each other fixture is 768 MiB with a unique GPT and
   a basic NTFS partition. The virtual AHCI target exposes 512-byte logical and
   physical sectors. Set the port's VirtualBox serial to the first 20 hexadecimal
   characters of the target VDI UUID after removing hyphens.
3. Boot WinPE, use `inventory` and `diagnose` to record the source and disk map,
   and create a transaction with `backup-start`. Use `backup-verify` to reread it.
   Retain source fixture hashes and independently read the published payload from
   the backup VDI. Set both source and backup read-only in WinPE and record the
   resulting states. Confirm they still contain the recorded source and backup.
4. Run `deployment-media` and export the reviewed `deployment-plan` using the
   optical WIM's SHA-256, the selected index and target GPT identity. Select
   `limpa` and omit the optional preservation-of-target argument. The verified
   backup in this procedure belongs to a different, protected source disk.
   Run `deployment-lab-observe` with the GPT identity. Shut down WinPE and
   extract the plan, observation and backup manifest to the host.
5. Run `scripts/wave5-attest.py --help` for the registration arguments. The host
   tool validates the active run, actual VM configuration, VDI header and GPT
   tables, serial, plan and backup identity before signing. Transfer
   `registration.json` and `registration.sig` into the selected evidence folder
   for GUI execution, or supply them on the auxiliary optical carrier to the
   QA CLI. Confirm the host and guest UTC clocks agree within the token window.
   If the VM or attachments change, use new observations and attestation.

## Execute and boot

In the QA GUI, use the existing media, disk, edition, backup and evidence
pickers. Review the clean plan and confirm the disk GPT identity and erase
notice. The equivalent QA CLI takes the exported plan, signed registration,
detached signature, exact operation token, verified backup directory and
evidence directory in that order.

The executor reserves the operation ID at the evidence-volume root. The
append-only `execution-NNN.json` records and tool streams survive reboots.
The last valid checkpoint shows the last confirmed step. An interrupted or
failed write requires manual inspection. Repeating the operation against a
populated target is blocked.

The supported layout is a 300 MiB FAT32 ESP, a 16 MiB MSR and an NTFS Windows
partition using the remaining available space. The initial laboratory profile
does not provision WinRE. The executor applies the reviewed WIM index with
DISM, confirms Windows markers, configures the verified ESP using BCDBoot,
reads the EFI files and enumerates the BCD store. The WinPE result is
`awaiting_first_boot`.

Shut down WinPE. Eject the two boot/install optical media and put the target
first in the UEFI boot order. Complete Windows setup with a disposable local
account. Keep credentials out of operation evidence and command transcripts.
Capture the running OS build, firmware mode, target GPT identity, signed-in
account SID and Explorer status.

## Restore and verify

Under the new account, run the Restore Agent's `perfil`. Use
`previa-recuperacao` with the final execution record, independently mounted
backup, SID, conflict policy and explicit mapping pairs for Desktop,
Documents, Downloads and Pictures. Confirm the preview fingerprint with
`restaurar-recuperacao`. The restore record carries the deployment operation
and backup-manifest binding.

Power down and extract read-only NTFS partition images from the three
disposable VDIs. Run `scripts/wave5-verify-restore.py --help` for the inputs.
The verifier reads source, backup and restored files independently, compares
counts, sizes and hashes, and checks the existing conflict and empty directory.
Native backup and restore regression tests require
`ARCHRESCUE_TEST_SOURCE` and `ARCHRESCUE_TEST_DESTINATION` on separate
persistent disposable GPT filesystems. Run the installed-Windows fixture suite
with an elevated test token for its read-only raw-device queries. The Restore
Agent itself runs under the destination account’s normal token.

Promote compact evidence and hash it. Verify VM and snapshot mappings,
unregister the disposable guest, close its media, then close the scratch run
through `scripts/scratch.py close <run-id> --cleanup`.

- [Microsoft UEFI/GPT partition requirements](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/configure-uefigpt-based-hard-drive-partitions?view=windows-11)
- [Microsoft WIM deployment procedure](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/capture-and-apply-windows-using-a-single-wim?view=windows-11)
- [Microsoft BCDBoot options](https://learn.microsoft.com/en-us/windows-hardware/manufacture/desktop/bcdboot-command-line-options-techref-di?view=windows-11)
