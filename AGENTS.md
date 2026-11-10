# ArchRescue repository instructions

Read [the source style guide](docs/STYLEGUIDE.md) before creating or modifying source code. Follow its conventions for logical spacing, descriptive names, multiline formatting, and English comments. Review the rendered source after formatting and verify that validation, processing, and results are easy to find. Treat `cargo fmt --all --check` as the formatting baseline and review readability separately.

Preserve the read-only source and device-query boundaries. Keep backup validation, storage identity checks, cancellation, and verification visible when editing those paths. Preserve the assumptions documented around unsafe code.

Run `scripts/test.sh` with `ARCHRESCUE_TEST_DESTINATION` set to a disposable directory on a separate filesystem. Cross-check Windows-only changes with an installed Windows target. Record separately any runtime checks that need Windows or WinPE.

Follow the [workspace storage policy](docs/WORKSPACE-STORAGE.md). Budget and register image-heavy work before it starts. Promote compact evidence, verify dependencies and clean owned scratch before closing the run. Use at least 15 GiB of free W10Val system-volume space for substantial Windows builds.
