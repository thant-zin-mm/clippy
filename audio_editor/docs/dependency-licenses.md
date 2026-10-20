# Dependency license audit

Binding for the whole build. Owner: whichever task adds the dependency.
Update this file **in the same commit** that introduces the dependency.

Our project is **GPL-3.0-only** (`LICENSE`). That is compatible with every
license below (all permissive, and MPL-2.0/LGPL with their notices kept).

## Currently in the tree (P0-T1, P1-T1)

Verified via `cargo metadata` (backend) and `npm` manifests (frontend). No
copyleft, no special obligations beyond attribution — except the MPL-2.0 and
LGPL rows below, which carry their own notices.

| Component | License | Obligation for us |
|---|---|---|
| axum, axum-core, hyper, hyper-util, tower, tower-http, matchit | MIT (matchit: MIT AND BSD-3-Clause) | Keep copyright notices |
| tokio, mio, socket2, signal-hook-registry | MIT | Keep copyright notices |
| http, http-body, http-body-util, httparse, httpdate, mime, bytes | MIT / MIT OR Apache-2.0 | Keep copyright notices |
| serde, serde_core, serde_json, serde_urlencoded, serde_path_to_error, itoa | MIT OR Apache-2.0 | Keep copyright notices |
| ryu | Apache-2.0 OR BSL-1.0 | Keep copyright notices |
| thiserror, thiserror-impl, syn, quote, proc-macro2 | MIT OR Apache-2.0 | Keep copyright notices |
| tracing, tracing-subscriber, tracing-core, matchers, nu-ansi-term, sharded-slab, thread_local, lazy_static, once_cell | MIT | Keep copyright notices |
| regex-automata, regex-syntax, aho-corasick, memchr | Unlicense OR MIT (we take MIT) | Keep copyright notices |
| libc, cfg-if, pin-project-lite, smallvec, slab, futures-*, multer, async-stream | MIT OR Apache-2.0 (async-stream: MIT) | Keep copyright notices |
| frontend: vite, vitest, eslint, typescript-eslint, prettier, typescript, @types/* | MIT / Apache-2.0 | Keep notices |
| **symphonia, symphonia-core, symphonia-codec-pcm, symphonia-format-riff, symphonia-metadata, symphonia-bundle-mp3** (added P1-T1) | **MPL-2.0** | See "symphonia" below — we ship an unmodified build and call its public API only |
| **mp3lame-encoder, mp3lame-sys** (added P1-T1 for P2-T1) | **LGPL-3.0** | See "lame" below — static vendoring of lame-3.100; source-build only in v1 |
| reqwest + HTTP/TLS transitive stack (hyper-tls, hyper-rustls, native-tls, openssl, rustls, ring, tokio-rustls, tokio-native-tls, encoding_rs, url, idna, icu-*, tempfile) — **dev-dependencies only** (tests) | MIT / Apache-2.0 / ISC / BSD-3-Clause / Unicode-3.0 / Zlib | Keep copyright notices; never linked into shipped code paths |

None of these are GPL-incompatible. No AGPL, no SSPL, no Commons Clause.

### symphonia (decode) — **MPL-2.0**
File-level weak copyleft (arch §7).

- **Allowed:** yes. MPL-2.0 is one-way-compatible with GPL-3.0.
- **Obligations:**
  - Keep symphonia's copyright + license notices.
  - Any **file** we modify *inside* symphonia must be published under
    MPL-2.0. Do not modify its sources — call its public API only. If a fix
    is genuinely needed, upstream it; if we must patch, publish the patched
    files under MPL-2.0.
  - Merely linking/calling it does **not** make our GPL code MPL-covered.
- **Our status:** added unmodified in P1-T1 (`symphonia` facade with the
  `mp3`, `wav`, `pcm` features); we only call its API. LICENSE/NOTICE files
  from the crates are preserved in any bundled distribution.

### lame / libmp3lame via `mp3lame-encoder` — **LGPL-3.0**
(arch §7; MP3 export lands in P2-T1. The crate was added early in P1-T1 so the
encoder is available when P2-T1 starts.)

- **Our choice:** `mp3lame-encoder 0.2` (LGPL-3.0), which vendors lame-3.100
  and links it **statically** — no system libmp3lame/headers required. The
  earlier `lame-sys` dynamic-linking plan was dropped (see plan.md P2-T1
  decisions); this section replaces it.
- **Obligations (static linking):** (a) ship the LGPL notice, (b) allow
  relinking against a modified lame (ship the library's object files, or a
  written offer), (c) no DRM/"effective technological measures" restrictions
  that stop relinking.
- **Note (D21 / arch §11):** v1 is **source-build only**. We do not ship
  binaries, which removes the "provide the library/notice with the binary"
  packaging work for now. These obligations become real only if we ever ship
  a release bundle (post-v1); the build docs must then record the relink path.
- **Our status:** in tree as of P1-T1, used starting in P2-T1 (export). The
  system `lame` **CLI** is used only by `scripts/gen-fixtures.mjs` at dev time
  and is not distributed.

## Checklist for the reviewer

- [ ] New dependency's SPDX license recorded in the table above.
- [ ] No AGPL/SSPL/copyleft-strong license added.
- [ ] If MPL-2.0: we call the API, do not modify its files.
- [ ] If LGPL: linking strategy documented (dynamic preferred; static OK with
      relink materials) + notice documented.
- [ ] `LICENSE` (GPL-3.0) and per-crate notices preserved in any bundle.
