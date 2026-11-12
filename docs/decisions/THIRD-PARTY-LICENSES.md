# Third-party dependency inventory

Generated from locked Cargo metadata. These are declared SPDX expressions, not a redistribution approval. Retain upstream notices when distributing. No dependency binaries were publicly distributed.

| Crate | Locked version | Declared license |
|---|---|---|
| arrayref | 0.3.9 | BSD-2-Clause |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| block-buffer | 0.10.4 | MIT OR Apache-2.0 |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| cpufeatures | 0.2.17 | MIT OR Apache-2.0 |
| crypto-common | 0.1.7 | MIT OR Apache-2.0 |
| digest | 0.10.7 | MIT OR Apache-2.0 |
| generic-array | 0.14.7 | MIT |
| itoa | 1.0.19 | MIT OR Apache-2.0 |
| libc | 0.2.191 | MIT OR Apache-2.0 |
| log | 0.4.35 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| roxmltree | 0.21.2 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.154 | MIT OR Apache-2.0 |
| sha2 | 0.10.9 | MIT OR Apache-2.0 |
| strict-num | 0.1.1 | MIT |
| syn | 3.0.10 | MIT OR Apache-2.0 |
| tiny-skia | 0.12.0 | BSD-3-Clause |
| tiny-skia-path | 0.12.0 | BSD-3-Clause |
| typenum | 1.20.1 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| version_check | 0.9.5 | MIT/Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| zmij | 1.0.23 | MIT |

The final ArchRescue source license remains an owner decision. Microsoft ADK/SDK/WinPE are external proprietary build prerequisites, absent from this repository. Windows API bindings are source crates; their licenses do not license the Windows runtime.

The Windows renderer enables tiny-skia with its standard library and CPU SIMD support. Its PNG codec and default feature set are disabled. The renderer dependencies use permissive licenses compatible with the local build kit. The corresponding notices accompany the local executable. GDI registers the bundled Geist and IBM Plex Mono TTFs in process memory. The installed fonts provide fallback. The [font asset directory](../../apps/boot-shell/assets/fonts) contains upstream provenance and both SIL OFL 1.1 notices. [tiny-skia source and license](https://github.com/linebender/tiny-skia)
