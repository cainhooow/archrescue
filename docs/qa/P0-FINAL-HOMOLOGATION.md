# P0 final homologation

## Verdict

**P0 BLOCKED**

Mandatory ISO construction and real VM execution did not occur. The user-visible Windows capabilities cannot be accepted on portable tests alone. See the [full feature matrix](P0-TEST-MATRIX.md).

## Review method

A separate adversarial review pass challenged device-response parsing, callback reentrancy, source/destination validation, stale-binary evidence, UI selection errors, report content and license claims. The CLI checker independently compares bytes/hashes using Python and requires the intended source build ID. This is independent verification tooling, **not a claim of review by a separate human or independent QA organization**. The implementation agent performed the code review; external independent sign-off remains outstanding.

## Findings corrected

- Windows handle typing / required API feature imports corrected after actual cross-target diagnostics.
- Raw device structures containing boolean fields replaced with bounded scalar parsing; malformed/truncated/count/bounds cases added.
- Win32 synchronous callback reentrancy protected with RefCell borrowing and separate paint state.
- Inaccessible selected explorer entries are passed to preflight rather than silently filtered out.
- Copy byte accounting handles short writes; completion remains gated by source/destination readback hashes.
- Embedded build identity added; a caught stale-executable verification attempt was superseded by a successful matching-source build and run.
- Report constructor failure corrected, without weakening lint/test assertions.
- Cancellation control is enabled only for cooperative backup cancellation; general diagnostics do not promise cancellation.
- Build script handles clean Git output as an array; still requires execution on a real Windows build host.

## Remaining blockers and risks

1. No MSVC linker/SDK, Windows runtime, ADK/WinPE, OVMF/QEMU or second VM platform available. Import-DLL compatibility, boot, fallback and native UI remain untested.
2. No real Windows virtual-disk test data was supplied or generated. Storage API identity mapping, BitLocker/EFS errors, read-only image behavior and persistent backup after reboot are unverified.
3. Source paths must be quiescent. Path prechecks are not hardened against hostile concurrent filesystem namespace changes. No hardware write blocking is provided.
4. Backup preserves content only. Partial files remain; an abrupt process/power loss can prevent a manifest from being saved. There is no resume protocol.
5. Candidate detection validates signatures/structure, not complete registry/kernel authenticity. Version/build detection remains unavailable. BCD reference consistency is not implemented; command output is evidence, not a health verdict. The synchronous Windows bcdedit query has no timeout and needs runtime hardening if a damaged store stalls it.
6. Localization covers the primary navigation/actions; raw technical evidence and structured errors are English. Full user-facing error translation and accessibility review remain outstanding.
7. The Windows build script and startup answer file were reviewed as source but not executed. ISO bitwise reproducibility is not claimed.
8. Final ArchRescue license selection and exact ADK/WinPE redistribution rights remain unresolved. No binary or ISO was published.

## Evidence-based conclusion

There is runnable source and a verified Linux executable for the portable slice, with 31 passing tests and a three-file independent copy/restoration check. That is measurable progress toward P0, not delivery of the mandatory bootable milestone. Qualification must continue on the intended Windows/WinPE environment before acceptance.
