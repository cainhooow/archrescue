# VirtualBox QA

## VM and media

| Item | Verified value |
|---|---|
| QA VM | ArchRescue-QA, UUID `699389da-cde1-47b0-b018-536667e0cfee` |
| Hypervisor | VirtualBox 7.2.22r175245, EFI64, 2 vCPUs, 2,304 MiB, VBoxSVGA, 64 MiB VRAM, disconnected NIC |
| Firmware hash | `VBoxEFI-amd64.fd` SHA-256 `e92f2b20f43b41f38522a9c9d0ddd3f29bb5dea46c8022acf5f5ecd5ffabf849` |
| QA snapshot | `ARCHRESCUE-QA-PRE-BOOT`, UUID `5dd9486d-965e-4b28-adfc-c1e0981dd97e` |
| Final DVD | SATA 0, ISO SHA-256 `83e8dad200d60b31a9186d7a5eeaf803de212a35b89e9e0860163350359626f0` |
| Source | SATA 1, immutable VDI base `42c37c44-156a-4751-8823-555d818f26ea`, 24 GiB GPT with NTFS and an unmounted 260 MiB FAT32 EFI partition |
| Destination | SATA 2, newly created 3 GiB VDI `58caa592-bf96-4022-a235-45ad25a09890`. Only this verified blank disk was initialized and formatted in the QA guest. Active snapshot child `919bc8e9-e3a7-4d4d-8f73-fcade9ef3590`. |

The source was a clone of the disposable disk prepared from verified Microsoft Windows 10 installation media. VirtualBox rejected its HDD `Readonly` mode. The immutable base protected the source image while the guest saw a disposable differencing disk. WinPE DiskPart also refused to set the disk read-only. The strict guest-visible write-blocking requirement remains BLOCKED. The test separately checked the immutable base hash and the three source payloads. [VirtualBox disk modes](https://docs.oracle.com/en/virtualization/virtualbox/7.2/user/storage.html)

## Boot and functional observations

Two early boot attempts selected the disposable source disk after the optical prompt timed out or the EFI boot order changed. The QA guest shut down normally, and its snapshot was restored. The final procedure boots from the verified DVD with the source detached, then hot-plugs the immutable clone for the storage test. A host suspension interrupted one attempt with the final ISO. The repeat was executed from the same verified optical media.

The first successful optical run used the BCD-fix source `9e624d2` at 1280×720. UEFI loaded `cdboot.efi`, WinPE initialized, and ArchRescue showed its pt-BR GUI. Closing the window exposed a defect: the GUI process stayed alive and held the fallback console. A console interrupt recovered that run for storage testing and produced exit `-1073741510`. The final `64bf3a6` ISO corrected teardown. At 800×600, the keyboard opened the Disks view and Alt+F4 returned to the fallback console normally. The startup log showed application exit 0 and the expected full Git SHA. The embedded metadata matched the final executable SHA-256.

| Capability | Result | Observation |
|---|---|---|
| UEFI boot, WinPE initialization and app startup | PASS | Final local ISO reached the visible ArchRescue UI and startup log. Loader resolved the exercised imports. |
| Keyboard and fallback console | PASS | Tab and Space selected Disks at 800×600. Alt+F4 closed the final app and returned to the console with exit 0. |
| Storage and offline detection | PASS | WinPE enumerated the 24 GiB source and 3 GiB destination, with GPT, NTFS identities and an unmounted FAT32 EFI partition. It distinguished the X: RAM disk and detected the offline x64 Windows candidate. |
| Browse and verified copy | PASS | Three files and 262,173 bytes, including a Unicode file and an empty file. The empty directory was retained. |
| Recovery output | PASS | An independent restoration to a third directory matched the source, copied files and post-run source payloads. |
| WinPE negative cases | PASS | The `9e624d2` CLI rejected same-volume, overlapping path, existing destination and missing volume with exit 1 and zero copied bytes. The final build repeated these checks natively on Windows. |
| Strict write blocking | BLOCKED | Immutable backing image with a writable differencing layer. VirtualBox and WinPE rejected a guest-visible read-only disk in this setup. |
| Full WinPE GUI recovery walkthrough | NOT TESTED | Navigation, lifecycle and console were observed. WinPE storage, copy and report were exercised through the product CLI. The native Windows GUI exercised copy, errors and cancellation. |
| QEMU and OVMF | BLOCKED | The required second platform and firmware were unavailable on the Arch host. |
| Secure Boot | NOT TESTED | The delivered ISO used standard boot media. A firmware trust and revocation test remains outstanding. |

The final ISO ran at both 800×600 and 1280×720. The first run stored the verified copy, restoration, report, startup log and build metadata on the distinct NTFS destination. After a graceful shutdown and the 1280×720 boot, the console listed both report files and the copied and restored files. The application reopened from the fallback console and closed again before the QA guest shut down. A second read-only VHD extraction verified that all 21 files retained their hashes across the reboot.

The final copy operation was `ar-18e69a75c7690d80-5d4-0`. The effective destination medium was cloned to a VHD and mounted read-only in W10Val. Independent SHA-256 manifests matched the pre-test source, verified copy, restoration and post-test source archive. The immutable source VDI's before and after SHA-256 was `5b2d9f769639069fff76a5e5a5ec6f43239ac50235854faa93415f2403315282`.

| File | Bytes | SHA-256 |
|---|---:|---|
| `empty.txt` | 0 | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `nested/payload.bin` | 262,144 | `6e69354705c1f9cd28e7fe8f48e4be374955eaa1666600d2764e5503ef30062b` |
| `nested/çao.txt` | 29 | `7a1acacd61e9e13d031e60fb5496ff4bfcaf784780ce0c13d3959af1034793b5` |

Final report SHA-256 values were `d16750f9030feeecf42bfb6d42e7af0e77d2cb20b343c278fde38bc34f984e15` for JSON and `5b0166661f2ff4df9cbe10798cbca76c64f27b980c5e50712f2037554e2bc7d5` for text. See `qa-final-independent-verification.log`, `qa-final-source-base-hash-after.log`, `qa-final-readback.log`, `qa-post-reboot-hashes.log`, `qa-final-export/`, and screenshots `final-1280-reboot-report.png`, `final-1280-reboot-copies.png` and `final-1280-reopen.png`.

## Local evidence

See `.agent-tmp/w10val/qa/` for VM configurations, firmware and boot logs, read-only VHD extraction logs, raw CLI JSON and screenshots. Key captures include `screenshots/optical-only-observed.png`, `screenshots/final-800-dashboard.png`, `screenshots/final-800-keyboard-navigation.png` and `screenshots/final-800-clean-close.png`. The fallback runs and rejected read-only requests are retained alongside the successful evidence.
