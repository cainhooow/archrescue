# Corrective wave acceptance

13–21 November 2026 · Overall **PARTIAL**

## Final continuation: 20–21 November

The final ordinary executable was built from
`9cb39cbd4f39e7a0ea531160211b7aa9b4388709`. Acceptance remains **PARTIAL**:
automatic EFI selection, guided ambiguity review, the missing-ESP refusal, the
original 800 × 600 action-row fix, and scrolling of long confirmation paths have
runtime evidence. Earlier live ESP identity/type changes exposed two stack
overflows during refresh on `f1e467a`. A complete replay on `9cb39cb` finished
without that failure; its cause has not been established or explicitly fixed.
It remains an unresolved intermittent failure rather than a final-build PASS
based on the automated suites.
UNC GUI coverage and complete Japanese glyph rendering remain limited by the
laboratory environment.

| Final artifact | Identity |
|---|---|
| Ordinary shell, Windows GNU x64 release, default features | [EXE](../../artifacts/windows/9cb39cb/archrescue.exe), SHA-256 `259314ce09c85925788023135cfc34e721b671f85dfa657adc367f93f052c03e` |
| Final QA carrier | SHA-256 `c3719a7fa57474c028c9ab902fb7e9f09c29c3fba29ac4710d10cd6b14dd1c1d`; contains the ordinary EXE, native test binaries and disposable-guest probes |
| Native test executables | Built at the same source revision with all features; [per-executable hashes](../../artifacts/evidence/corrective-wave/closure/accepted-native-binaries.sha256) |
| Guest identity and independent EXE hash | [Installed version](../../artifacts/evidence/corrective-wave/closure/9cb39cb-build.txt), [guest SHA-256](../../artifacts/evidence/corrective-wave/closure/9cb39cb-hash.txt), [WinPE version](../../artifacts/evidence/corrective-wave/closure/9cb39cb-native-build.txt) |

The continuation added three local commits. `a78a85a` explains why ambiguous ESP
candidates cannot authorize a repair and offers another discovered Windows
installation in the same picker. `f1e467a` discards the previous preview before
target revalidation starts. `9cb39cb` exposes scrolling in confirmation path
fields, including the tail of a path longer than 260 characters. No storage write
boundary or repair authorization gate was relaxed. No push or publication was
performed. The Restore Agent and signed deployment release were not rebuilt;
their runtime results below remain explicitly tied to `62a97e8`.

| Requirement | Runtime evidence and practical result | Final assessment |
|---|---|---|
| Automatic EFI discovery | The final build identifies the ESP after choosing Windows, without typing or selecting an EFI path. Switching from the ambiguous disk to the other discovered Windows identifies `S:\` on disk 2, partition 1. [Guided selection](../../artifacts/evidence/corrective-wave/closure/9cb39cb-guided-choices.png), [automatic result](../../artifacts/evidence/corrective-wave/closure/9cb39cb-guided-auto-efi.png) | PASS for the tested unique, readable same-disk GPT/FAT32 association; BCD association remains unverified |
| Multiple ESPs and guided refusal | Both physical ESP candidates are listed. Selecting a candidate for inspection leaves confirmation disabled; another discovered Windows can be chosen. Preview and BCDBoot remain unavailable while the source is ambiguous. [Blocked page](../../artifacts/evidence/corrective-wave/closure/9cb39cb-ambiguous.png), [candidate refusal](../../artifacts/evidence/corrective-wave/closure/9cb39cb-candidate-blocked.png) | PASS for guided review and refusal. There is no override that turns an unproved ESP candidate into an authorized target |
| Missing ESP | A fresh boot with the retained no-ESP fixture identifies Windows and reports “Nenhuma ESP no disco deste Windows”; both repair actions remain disabled. Refresh preserves the refusal. Source and destination whole-VDI digests match during the isolated refresh window. [GUI](../../artifacts/evidence/corrective-wave/closure/9cb39cb-missing-esp.png), [before](../../artifacts/evidence/corrective-wave/closure/missing-before.sha256), [after](../../artifacts/evidence/corrective-wave/closure/missing-after.sha256) | PASS |
| Changed partition identity and duplicate identities | On `9cb39cb`, a fixture-only probe changed the ESP GPT ID; refresh discarded the previous preview and identified the current target. Duplicating the ESP ID with the Windows partition ID offlined the fixture; the shell revoked the Windows selection and disabled repair. Resetting the ID, bringing the disk online, changing the ESP type to Basic Data and restoring EFI produced the expected missing/identified states without a crash in this replay. Two earlier `f1e467a` refresh overflows remain unexplained. [Changed ID](../../artifacts/evidence/corrective-wave/closure/9cb39cb-live-identity-change.png), [duplicate refusal](../../artifacts/evidence/corrective-wave/closure/9cb39cb-live-duplicate.png), [changed type](../../artifacts/evidence/corrective-wave/closure/9cb39cb-live-type-change.png), [restored EFI](../../artifacts/evidence/corrective-wave/closure/9cb39cb-live-restore-result.png), [earlier overflow](../../artifacts/evidence/corrective-wave/closure/refresh-stack-overflow.png) | PARTIAL: final replay passed these transitions; the earlier intermittent overflow is not closed |
| Physical replacement while running | Hot removal of the disposable source caused WinPE `INTERNAL_POWER_ERROR` before a replacement could be evaluated by the app. [Guest failure](../../artifacts/evidence/corrective-wave/closure/hot-removal-guest-crash.png), [VM configuration](../../artifacts/evidence/corrective-wave/closure/hot-removal-crash-vm.txt) | NOT ACCEPTED as an application refusal test |
| Assisted paths across the shell | Runtime sweeps cover all ten navigation screens at 800 × 600 in WinPE and 1024 × 768 in installed Windows, plus 33/55 diagnosis details, 8/16 disk details, and the WinPE integrity blocker. The 145 captured frames and 141 assisted control-text dumps (plus two build-identity records) contain no recognized raw internal namespace prefixes. Technical console/report views retain the exact prefixes; returning to assisted mode removes them again. [Scan and counts](../../artifacts/evidence/corrective-wave/closure/runtime-path-scan.json), [WinPE controls and frames](../../artifacts/evidence/corrective-wave/closure/9cb39cb-ui-winpe/pages/page-101.txt), [cached decision after switching back](../../artifacts/evidence/corrective-wave/closure/9cb39cb-ui-winpe/report-return.png), [readable report](../../artifacts/evidence/corrective-wave/closure/9cb39cb-report.txt), [exact JSON](../../artifacts/evidence/corrective-wave/closure/9cb39cb-report.json) | PASS for the exercised local-volume surfaces; UNC remains unaccepted |
| Responsive actions and long paths | At 800 × 600 the selected-source backup screen exposes all four actions. A 299-character Unicode file path was selected through the picker. Its confirmation has visible scrolling and the final path component can be reached with the mouse. [Action row](../../artifacts/evidence/corrective-wave/closure/9cb39cb-backup-actions.png), [scroll control](../../artifacts/evidence/corrective-wave/closure/final-long-confirmation-scroll.png), [visible tail](../../artifacts/evidence/corrective-wave/closure/final-long-confirmation-tail.png) | PASS for these viewports and states; other DPI/viewport combinations are not claimed |
| Unicode and network limitations | An `f1e467a` GUI run published and verified the new 19-byte long-path file. The final `9cb39cb` confirmation was replayed, scrolled and declined. Portuguese characters render; Japanese characters survive in filenames/control text but display as missing glyphs in this WinPE font environment. A real UNC attempt failed in Windows with network error 1231 while NICs were disabled, before a GUI source could be exercised. [Completed backup](../../artifacts/evidence/corrective-wave/closure/long-unicode-completed.png), [fixture](../../artifacts/evidence/corrective-wave/closure/long-path-fixture.txt), [UNC runtime](../../artifacts/evidence/corrective-wave/closure/unc-live.png), [preparation result](../../artifacts/evidence/corrective-wave/closure/unc-prep.txt) | PARTIAL; no claim of complete CJK rendering or UNC GUI acceptance |

### Final automated gates

The earlier **76 WinPE + 76 installed-Windows** results at `a5f5e73` are retained
unchanged. After the continuation's source changes, both complete native suites
were built and run again at **`9cb39cb`: 76 passed in each environment**, with zero
failed or ignored tests, twelve per-binary numeric exits of 0 and `suite.exit=0`.
These totals come from the actual logs, not from a static test inventory.

- [Final WinPE log](../../artifacts/evidence/corrective-wave/closure/9cb39cb-winpe-native.log) and [actual fixture inventory](../../artifacts/evidence/corrective-wave/closure/9cb39cb-winpe-native-inventory.json).
- [Final installed-Windows log](../../artifacts/evidence/corrective-wave/closure/9cb39cb-installed-native.log) and [actual fixture inventory](../../artifacts/evidence/corrective-wave/closure/9cb39cb-installed-native-inventory.json).
- [`scripts/test.sh`](../../artifacts/evidence/corrective-wave/closure/accepted-linux-gates.log): 69 Linux tests passed with a disposable tmpfs destination and serial execution; formatting and Clippy passed. The first concurrent attempt was interrupted after another fixture's cleanup changed free space during the insufficient-space test and started a large copy. Its [log](../../artifacts/evidence/corrective-wave/closure/linux-first-concurrent.log) is retained; it is not counted as a pass.
- Windows [GNU](../../artifacts/evidence/corrective-wave/closure/accepted-gnu-clippy.log) and [MSVC](../../artifacts/evidence/corrective-wave/closure/accepted-msvc-clippy.log) Clippy checks passed with installed targets. [Release build](../../artifacts/evidence/corrective-wave/closure/accepted-release-build.log) and [native build](../../artifacts/evidence/corrective-wave/closure/accepted-native-build.log) succeeded. Windows/WinPE runtime checks are recorded separately from these compile checks.

The installed runner used the elevated fixture console; the native log contains
high-integrity SID `S-1-16-12288`. WinPE used its elevated recovery console;
`whoami.exe` is absent and its probe returned 9009. No substantial build was run
inside either Windows guest. The deployed system volume measured
66,036,350,976 bytes free, above the 15 GiB build floor. The historical W10Val
clone below that floor was not used for a new build.

### Protected content and measurement boundaries

The final independent offline readback still matches **six files and 12,543,360
bytes**, including source, backup and restored SHA-256/size identities, the empty
directory, the preserved conflict and its renamed restored copy.
[Final readback](../../artifacts/evidence/corrective-wave/closure/final-payload-readback.json).

The three protected lab VDIs also match exactly across the final build's sampled
Technical/assisted GUI window: [before](../../artifacts/evidence/corrective-wave/closure/9cb39cb-protected-before.sha256)
and [after](../../artifacts/evidence/corrective-wave/closure/9cb39cb-protected-after.sha256).
The missing-ESP refresh window independently matches both source and destination.
In the identity-mutation windows, the source fixture's digest matches after the
deliberate mutation; the evidence destination differs because it receives probe
logs and filesystem metadata. These are scoped operation-window results.

Whole-VDI digests **do differ** across the larger boot/mount/shutdown interval;
the [comparison](../../artifacts/evidence/corrective-wave/closure/protected-full-interval.diff)
is retained. There is no whole-session zero-write claim. Fixture GPT mutations
were intentional external QA actions, restricted by the probe to the registered
1 GiB disk GUID and named test partitions. They are excluded from application
no-write measurements. No real user disk was a test target.

Independent [final GPT readback](../../artifacts/evidence/corrective-wave/closure/independent-final-topology.json)
also proves that the Windows partition ID stayed unchanged, the ESP ID changed,
and its type was restored to EFI. The new Unicode/long-path backup independently
matches its source and manifest at 19 bytes and SHA-256
`71d51df4c56e052aa3fa13ab0a51d39c8242c8f684e799314cb619d77732fd49`.
[Filename and payload readback](../../artifacts/evidence/corrective-wave/closure/long-backup-readback.json).

### Controlled cleanup and retained reproducer

All registered lab media were preserved through runtime collection, independent
readback, promotion and evidence-link verification. Cleanup then unregistered the
disposable installed-Windows VM and deleted 51 unused media through VirtualBox
after checking that they had no VM or child dependency. Bulk extraction images,
superseded carriers, task-owned tools and other owned scratch were removed.
[Cleanup result and exact removed paths](../../artifacts/evidence/corrective-wave/closure/cleanup-result.json).

Run allocation fell from **56,999,911,424 to 594,477,056 bytes**. Measured system
filesystem free space rose from **213,190,025,216 to 262,605,762,560 bytes**,
an increase of about **46.0 GiB**. Allocation and actual free-space changes are
reported separately because the filesystem can compress or share extents.
The interrupted Linux fixture was also removed, returning 2,466,578,432 bytes to
tmpfs; the serial test destination and task-created Python cache were cleaned.
[Separate scratch cleanup](../../artifacts/evidence/corrective-wave/closure/independent-scratch-cleanup.json).

The scratch run is now explicitly **retained**, with a **1 GiB budget** and a
recorded reason: preserve a compact replay fixture until the earlier intermittent
refresh failure is understood. It contains only the powered-off corrective VM,
three disposable VDIs and the final QA carrier, about **567 MiB allocated**.
No VM remains running. [Final manifest](../../artifacts/evidence/corrective-wave/closure/scratch-final.json),
[fixture, identifiers and replay recipe](../../artifacts/evidence/corrective-wave/closure/REPRODUCER.md).
This is a controlled retention, not a claim that all laboratory storage was
deleted.

The two original P0 VM configuration hashes and all external hard-disk registry
identities/dependencies were preserved. Removing the disposable installed VM
automatically released two unused DVD registrations (Windows setup and Guest
Additions); those external ISO files were not deleted. The WinPE and Windows
setup ISO hashes still match their recorded identities.
[Protected configuration hashes](../../artifacts/evidence/corrective-wave/closure/protected-configurations-after-cleanup.json),
[released DVD registration record](../../artifacts/evidence/corrective-wave/closure/automatically-released-dvd-registrations.json),
[external ISO verification](../../artifacts/evidence/corrective-wave/closure/external-iso-integrity-after-cleanup.json).
The original W10Val disks, snapshots, installed SDK/ADK, Rust and other tools were
not cleanup targets. The existing shared Cargo cache was preserved.

### Remaining acceptance work

Investigate the earlier live EFI refresh overflows and repeat stress acceptance,
validate physical replacement
in an environment that survives the operation, and exercise a reachable UNC
source and suitable CJK fonts. Historical authorized BCDBoot repair/boot,
independent GUI execution of signed deployment, the complete multiple-Windows
integrity walk, and the remaining cancellation/refusal combinations below are
still open. A passing native suite does not close any of these UI/UX or runtime
requirements.

## Preserved earlier-wave evidence

The matrix below records the evidence as it stood before this continuation.
Its `a5f5e73` “final” build is the prior final candidate, not the current EXE.

The clean starting checkout was `main` at
`986f570f690b02880a6a51a6085d90c1735366c7`, equal to the audit baseline.
The initial corrective source is `62a97e8d4d1ed51d4692a9daea46b11c5d8b8fce`.
The final source is `a5f5e73a30e06476a720df8e5ccbe0ba89e55e9e`. The follow-up
at `ef387b9` keeps backup actions visible at the minimum viewport and localizes
disk-separation refusals. The two subsequent changes correlate ESP drive mounts
that WinPE omits from volume listings and preserve a verified volume root when
WinPE normalizes it to a folder mount. The commits and evidence are local.

| Build or medium | Identity |
|---|---|
| Baseline | Windows GNU x64 release, default features. SHA-256 and version in [baseline evidence](../../artifacts/evidence/corrective-wave/baseline.sha256) |
| Ordinary corrective shell | Windows GNU x64 release, default features, SHA-256 `637d1f37ccdad3722faf5321af63f7e2b339206696f789ea7ca5dba6e0bc0c29` |
| Lab corrective shell | Same source and target with `lab-deployment` and a per-run public key, SHA-256 `ebfb21c811a740bd3773775327252c38c59232d231bb76ae985ac539bca3c54a` |
| Installed Restore Agent | Same source, Windows GNU x64 release, SHA-256 `548ffcdfbe040974be680b368b8caff967ecd0581c48983a81ecbd832402c397` |
| Minimum viewport follow-up | `ef387b9`, Windows GNU x64 release, default features, SHA-256 `7549ba79b5517f459bceb9e22efcb25ac0c33a991b988f48715748aa5b215db5` |
| Final ordinary shell | `a5f5e73`, Windows GNU x64 release, default features, SHA-256 `100dc5e61d21e856f6b1e75540d964481b8afeb40e5278496ef628de5698d3d4` |
| WinPE boot ISO | Historical 7355215, SHA-256 `f59deb23036d17263fc58bc4c1fc90cca26f25da0e44f6421355bf0aace0b880`, running WinPE 10.0.26100.1 |
| Installation media | Mounted pt-BR Windows 10 22H2 ISO, SHA-256 `0ca714572fcb5b8ce480acc3c93eaf2f34cc6507df5f693cff021bf4a32fac68` |

[Environment](../../artifacts/evidence/corrective-wave/environment.log) · [Final executable hashes](../../artifacts/evidence/corrective-wave/final-binaries.sha256) · [Ordinary VM and attachments](../../artifacts/evidence/corrective-wave/final-vm.txt) · [Lab VM and attachments](../../artifacts/evidence/corrective-wave/lab-vm-plan.txt)

## Matrix

| Requirement and entry point | Fixture and expected result | Observed result | Status |
|---|---|---|---|
| Baseline Repair on entry | Single offline Windows and readable letterless FAT32 ESP, same GPT disk. Identify before requesting preservation | Baseline said “Identificação pendente · consultar ESP.” [Screenshot](../../artifacts/evidence/corrective-wave/baseline-repair.png) · [Controls](../../artifacts/evidence/corrective-wave/baseline-repair-controls.txt) | FAIL |
| Baseline assisted diagnosis | Select the recovery-files finding for the synthetic `ReAgent.xml`. Show ordinary drive path | The detail contained an escaped device namespace. This is the reproduced route. The owner's original route remains unknown. [Screenshot](../../artifacts/evidence/corrective-wave/baseline-path.png) · [Controls](../../artifacts/evidence/corrective-wave/baseline-path-controls.txt) | FAIL |
| Baseline Checkpoints → Resume | No destination selected. Choose the discovered operation and proceed to confirmation | The generic picker returned to Save Files with Resume disabled. [Picker](../../artifacts/evidence/corrective-wave/baseline-resume-picker.png) · [After selection](../../artifacts/evidence/corrective-wave/baseline-resume-after-selection.png) | FAIL |
| Repair letterless identification | Ordinary shell. Disk 0, ESP partition 1, Windows NTFS partition 2. Identify without ESP interaction or preservation destination | The page identified both partitions on entry and stated the BCD association was unverified. [Screenshot](../../artifacts/evidence/corrective-wave/final-repair-letterless.png) | PASS |
| Repair lettered ESP and aliases | Final ordinary shell. One FAT32 ESP accessible as `S:\` and optionally by a folder mount. Identify one physical partition and retain the drive name | `ef387b9` lost the ESP when the folder mount was present. `a5f5e73` displayed the same `S:\` and GPT partition before and after adding the folder mount. Preview succeeded. Source/ESP and preservation destination VDI hashes matched across the sampled preview window. [Before fix](../../artifacts/evidence/corrective-wave/topology-alias-before.png) · [Lettered](../../artifacts/evidence/corrective-wave/topology-lettered.png) · [With alias](../../artifacts/evidence/corrective-wave/topology-alias.png) · [Preview](../../artifacts/evidence/corrective-wave/topology-alias-preview.png) · [Hashes](../../artifacts/evidence/corrective-wave/topology-alias-preview-diff.log) | PASS |
| Repair preview and destination | Same fixture. Refuse same-disk destination, accept independent disk, generate preview and retain the execution block | Picker refused Disk 0. Preview on Disk 2 displayed the ESP and the BCDBoot drive-letter prerequisite. [Refusal controls](../../artifacts/evidence/corrective-wave/final-same-disk-controls.txt) · [Preview](../../artifacts/evidence/corrective-wave/final-repair-preview.png) | PASS |
| Root and nested EFI reads | Ordinary CLI corroboration on WinPE. Read the GUID root and the nested boot directory | Both browse commands exited 0. The boot inspection read the intentionally invalid BCD and reported the invalid signature. [Exit codes](../../artifacts/evidence/corrective-wave/final-build-id.txt) · [Nested browse](../../artifacts/evidence/corrective-wave/final-esp-nested.json) · [BCD observation](../../artifacts/evidence/corrective-wave/final-esp-boot.json) | PASS |
| Repair missing, foreign and ambiguous topology | Reject a source disk with no ESP, foreign ESP, inaccessible or incompatible ESP, multiple ESPs and stale identities | The no-ESP fixture reported missing. Final-build variants displayed blockers for the NTFS ESP, multiple ESPs and a foreign-disk ESP. Changing the identified ESP type and refreshing discarded the preview. Relevant VDI digests matched across the sampled discovery and refresh windows. Physical replacement with duplicated identities remains open in the GUI. [Missing](../../artifacts/evidence/corrective-wave/final-repair-missing.png) · [Incompatible](../../artifacts/evidence/corrective-wave/topology-incompatible.png) · [Multiple](../../artifacts/evidence/corrective-wave/topology-ambiguous.png) · [Foreign](../../artifacts/evidence/corrective-wave/topology-foreign.png) · [Changed](../../artifacts/evidence/corrective-wave/topology-changed.png) | PARTIAL |
| Historical BCDBoot repair and boot | Registered historical clone, verified data and full EFI preservation, authorized repair, postchecks and observed boot | Requires a separate disposable clone and full runtime evidence | NOT TESTED |
| Assisted diagnosis and Technical switch | Ordinary build. Select the recovery-files finding, switch to Technical and back | Assisted screenshots display `C:\Windows\System32\Recovery\ReAgent.xml`. Technical exposes the raw evidence. [Assisted](../../artifacts/evidence/corrective-wave/final-path-assisted.png) · [Technical](../../artifacts/evidence/corrective-wave/final-path-technical.png) · [Returned](../../artifacts/evidence/corrective-wave/final-path-return.png) | PASS |
| Default readable report | Ordinary CLI export on an independent disk. Render friendly text and preserve JSON | Independently extracted `report.txt` had zero namespace matches. JSON retained exact paths and the diagnostic finding. [Scan](../../artifacts/evidence/corrective-wave/path-scan.log) · [Text](../../artifacts/evidence/corrective-wave/final-report.txt) · [JSON](../../artifacts/evidence/corrective-wave/final-report.json) | PASS |
| All assisted surfaces | Windows and EFI details, cached errors, decisions, console, exports, Unicode and long names, UNC and unnamed volumes | Automated formatter, report and mode coverage plus selected WinPE observations. Full control-text sweep, Technical refusal history, and actual UNC and long-path GUI fixtures remain open | PARTIAL |
| Detectar → Diagnosticar → Integridade | Shared Windows source and disabled reasons, multiple installations and remapping | Single source shared in WinPE. Stale selection and remapping have native test coverage. Full multiple-Windows and Integrity walk remains open | PARTIAL |
| Save Files → Resume → Verify | Baseline backup created at `W:\`, then source remapped to `C:\`. Choose checkpoint without typing and verify source plus published file | Ordinary shell resolved `C:\`, opened the explicit confirmation and verified the 15-byte payload with one reused file. A lab-shell run separately selected four profile folders and published six verified files. [Resume](../../artifacts/evidence/corrective-wave/final-resume-confirm.png) · [Verification](../../artifacts/evidence/corrective-wave/final-resume-verified.png) · [GUI backup](../../artifacts/evidence/corrective-wave/lab-gui-backup-completed.png) | PASS |
| Deployment review and registration refusal | Lab shell, Windows 10 Pro index 4, empty 80 GiB target, selected GUI backup and evidence disk. Require signed registration before destructive confirmation | GUI produced and exported the clean plan. Missing registration and an out-of-window clock were refused before confirmation. [Plan](../../artifacts/evidence/corrective-wave/lab-plan.json) · [Missing](../../artifacts/evidence/corrective-wave/lab-gui-registration-missing.png) · [Expired or mismatched](../../artifacts/evidence/corrective-wave/lab-signed-review.png) | PASS |
| Signed GUI deployment and target boot | Lab build from `62a97e8`. Review the selected physical disk and erase warning, authorize, observe progress, and boot with WinPE and setup media removed | Final journal shows `awaiting_first_boot`. DiskPart, DISM, BCDBoot and BCDEdit each exited 0. Target boot reached OOBE and the pt-BR Windows 10 Pro desktop. [Confirmation](../../artifacts/evidence/corrective-wave/lab-signed-confirmation.png) · [Progress](../../artifacts/evidence/corrective-wave/lab-gui-running.png) · [Journal](../../artifacts/evidence/corrective-wave/lab-journal/execution-018.json) · [Boot configuration](../../artifacts/evidence/corrective-wave/lab-first-authoritative-boot-vm.txt) · [OOBE](../../artifacts/evidence/corrective-wave/lab-boot-oobe.png) | PASS |
| Installed deployment-bound Restore Agent | Select the backup and matching final record, derive the ordinary account and four Known Folders, retain the existing conflict, and verify the payload | The agent ran at medium integrity and reported six verified files, zero skipped and five folders. Offline readback matched 12,543,360 bytes by file size and SHA-256 across source, backup and restored content. It also confirmed the empty folder, the preserved existing file and the renamed restored copy. This deployment fixture used ASCII filenames. The GUI required visual selection of the final record. [Review](../../artifacts/evidence/corrective-wave/restore-review.png) · [Success](../../artifacts/evidence/corrective-wave/restore-success.png) · [Readback](../../artifacts/evidence/corrective-wave/restore-readback.json) · [Account](../../artifacts/evidence/corrective-wave/installed-restore/token.txt) | PASS |
| Standalone Restore Agent | Select backups without a deployment record. Skip existing files, then restore a new Unicode file from a separate fixture | GUI reported zero restored and six skipped files with the original backup. A separate standalone run restored `revisão_日本語.txt` with process exit 0 under the ordinary account. Offline checks agreed on its filename, 33 bytes and SHA-256. [Skip result](../../artifacts/evidence/corrective-wave/restore-standalone-success.png) · [Unicode success](../../artifacts/evidence/corrective-wave/restore-unicode-success.png) · [Unicode readback](../../artifacts/evidence/corrective-wave/standalone-unicode/readback.json) | PASS |
| Native automated gates | Final source. Linux separate tmpfs, x64 Windows GNU/MSVC Clippy, native tests on disposable persistent GPT/NTFS | Linux 69 default and 73 all-feature tests passed. Both Clippy runs exited 0. WinPE and installed Windows each passed 76 tests, with zero failures or ignored tests, 12 numeric per-binary exit codes, and all 14 Wave 4 cases accounted for. [Linux](../../artifacts/evidence/corrective-wave/final-efi-linux.log) · [All features](../../artifacts/evidence/corrective-wave/final-efi-all-features.log) · [GNU](../../artifacts/evidence/corrective-wave/final-efi-gnu.log) · [MSVC](../../artifacts/evidence/corrective-wave/final-efi-msvc.log) · [WinPE](../../artifacts/evidence/corrective-wave/final-efi-native-winpe.log) · [Installed Windows](../../artifacts/evidence/corrective-wave/final-efi-native-installed.log) · [Wave 4 crosswalk](../../artifacts/evidence/corrective-wave/wave4-failure-reruns.tsv) | PASS |
| Refusal and cancellation matrix | Swaps, duplicate identities, protected media, permissions and space, invalid backup and media, cancellation, process and post-verification failures | Focused backend cases and selected GUI refusals. Remaining combinations and independent operation-window checks stay open | PARTIAL |
| Independent reviewer replay | Another reviewer follows the critical routes on the recorded builds and fixtures | Reviewer reproduced friendly/Technical switching, missing ESP and guided backup selection on the original build at 800 × 600, then verified the layout and refusal fixes on `ef387b9`. The reviewer cross-checked the deployment journal and replayed the installed deployment-bound restore with six skips and exit 0. Post-replay readback matched the original. Independent GUI execution of signed deployment remains open. [Replay](../../artifacts/evidence/corrective-wave/reviewer/REPLAY.md) · [Post-replay readback](../../artifacts/evidence/corrective-wave/restore-post-review-comparison.log) | PARTIAL |

## Reproduction and test environment

- Build the baseline from `git archive 986f570f690b02880a6a51a6085d90c1735366c7`. Set `ARCHRESCUE_BUILD_ID` to the full source commit and use `cargo build --locked --release --target x86_64-pc-windows-gnu -p archrescue-boot-shell`. Build the corrective shell and agent with the same target from the corrective source commit. The lab command adds `--features lab-deployment` and the public key from the runbook.
- Boot the retained WinPE ISO in a dedicated UEFI VM and attach the release executable on an auxiliary ISO. The baseline setup script provisions three 768 MiB disposable disks with Windows markers, the FAT32 ESP and its invalid BCD fixture, backup storage and evidence storage. The first layout assigned `W:`, `R:` and `L:`; WinPE re-enumeration assigned `C:`, `D:` and `E:`. Run the supplied UI probe only on the disposable guest. [Fixture setup](../../artifacts/evidence/corrective-wave/setup.cmd) · [DiskPart](../../artifacts/evidence/corrective-wave/setup.txt) · [UI probe](../../artifacts/evidence/corrective-wave/ui-probe.c)
- In `Reparar`, observe the ESP before selecting the preservation destination. Select `C:` in the destination picker to see the physical-disk refusal, then `E:` and generate the preview. The nested reproduction uses `browse` on the GUID root and on `EFI\Microsoft\Boot`. In `Salvar meus arquivos`, use `Retomar`, select the existing checkpoint and confirm after reviewing the rediscovered `C:` origin. [WinPE commands](../../artifacts/evidence/corrective-wave/final-qa.cmd) · [Repair pointer route](../../artifacts/evidence/corrective-wave/repair-walk.cmd)
- Linux used `ARCHRESCUE_TEST_DESTINATION=/dev/shm/archrescue-corrective.d836Ur`. The native reruns used a 1 GiB GPT/NTFS source and a 384 MiB GPT/NTFS destination. WinPE ran in the elevated recovery console with `C:\qa-source` and `D:\qa-destination`. The retained WinPE lacks `whoami.exe`; that probe returned 9009. Installed Windows 10.0.19045.6456 ran from a flattened disposable W10Val clone, with `E:\qa-source` and `F:\qa-destination` under the elevated SSH fixture account, integrity SID `S-1-16-12288`. The first runs used source volumes too small to allocate the insufficient-space test file. Both complete native reruns used the corrected fixture. The clone measured 13,022,048,256 bytes free on C: and no build was run inside it. [Runner](../../artifacts/evidence/corrective-wave/native.cmd) · [Installed token and space](../../artifacts/evidence/corrective-wave/installed-token-space.log)
- On `ef387b9`, set `ARCHRESCUE_BUILD_ID` to the full commit and run `cargo test --workspace --all-features --target x86_64-pc-windows-gnu --locked --no-run --message-format=json`. Place the executable test artifacts under `tests` on an auxiliary ISO alongside the final ordinary executable and `native.cmd`. The WinPE command was `F:\native.cmd D:\final-native-winpe C:\qa-source D:\qa-destination`. The installed rerun used the deployed Windows 10.0.19045.2965 and independent clones of the two persistent test disks. The elevated command was `J:\native.cmd K:\final-native-installed I:\qa-source K:\qa-destination`. [Executable hashes](../../artifacts/evidence/corrective-wave/final-native-binaries.sha256) · [Build](../../artifacts/evidence/corrective-wave/final-native-build.log)
- Repeat that build and fixture sequence at `a5f5e73`. Use `D:\a5f5e73-native` and `K:\a5f5e73-native` as the WinPE and installed log directories. The installed console used high integrity SID `S-1-16-12288`. The WinPE recovery console was elevated and the `whoami` probe returned 9009. [Executable hashes](../../artifacts/evidence/corrective-wave/final-efi-native-binaries.sha256) · [Build](../../artifacts/evidence/corrective-wave/final-efi-native-build.log) · [Carrier and fixture roles](../../artifacts/evidence/corrective-wave/final-efi-native-installed-vm.txt)
- The extra topology fixtures use the disposable Windows NTFS partition read back from the lab source, fresh GPT layouts, and FAT32 or NTFS ESP test filesystems. Assign `S:\` and add the folder mount in the fixture shell before launching the app. For stale preview, change the disposable ESP's GPT type to Basic Data while the preview remains open, then click **Atualizar**. Sample the host's two VDI files with the same running VM paused before and after GUI discovery, preview or refresh. [Fixture preparation](../../artifacts/evidence/corrective-wave/prepare-topology.sh) · [WinPE runner](../../artifacts/evidence/corrective-wave/topology.cmd) · [Rejected cases](../../artifacts/evidence/corrective-wave/walk-topology.sh) · [Refresh comparison](../../artifacts/evidence/corrective-wave/topology-changed-diff.log)
- In the lab, the GUI handled Detectar, profile-folder backup, plan review, media and target selection, registered execution and progress. The CLI captured inventory and observed identities for signing. The host copied the signed registration to the chosen evidence volume. A non-destructive preflight refusal had identical VDI digests during its sampled window. Across the later confirm-to-shutdown comparison, the backup VDI digest matched and the source and deployed target VDI digests differed. Independent readback establishes the tested file content after reboot. [Refusal comparison](../../artifacts/evidence/corrective-wave/lab-preflight-window.diff) · [Before confirmation](../../artifacts/evidence/corrective-wave/lab-confirm-window-before.sha256) · [After shutdown](../../artifacts/evidence/corrective-wave/lab-post-execute-delayed.sha256)
- On the installed desktop, run the supplied launcher under the destination account. The launcher creates the conflict fixture and records the token, profile, and process exit. In the agent, select `manifest.json` on the backup disk, choose the deployment route, then select `execution-018.json` on the evidence disk. Accept the mappings for Desktop, Documents, Downloads and Pictures, choose to keep both names, and confirm the review. After shutdown, run `extract-partition.py` on source partition 2, backup partition 1 and target partition 3. Pass the manifest and those three NTFS images to `compare-restore.py`. [Launcher](../../artifacts/evidence/corrective-wave/prepare-installed-restore.ps1) · [Extraction](../../artifacts/evidence/corrective-wave/extract-partition.py) · [Comparison](../../artifacts/evidence/corrective-wave/compare-restore.py) · [Process exit](../../artifacts/evidence/corrective-wave/installed-restore/restore-gui.exit)
- To reproduce standalone Unicode acceptance, create the sample backup on the two enlarged test volumes with the preparation script. Launch the agent as `RestoreQA` and visually select the resulting manifest under `K:\unicode-backup`. Choose the independent backup route, accept the Documents mapping, select the conflict policy and confirm. Read back partition 1 of both test disks and partition 3 of the deployed target with the comparison script. [Preparation](../../artifacts/evidence/corrective-wave/unicode-fixture.ps1) · [Launcher](../../artifacts/evidence/corrective-wave/unicode-restore.ps1) · [Comparison](../../artifacts/evidence/corrective-wave/compare-unicode.py)
