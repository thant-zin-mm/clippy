# Dependency license audit

Binding for the whole build. Owner: whichever task adds the dependency.
Update this file **in the same commit** that introduces the dependency.

Our project is **GPL-3.0-only** (`LICENSE`). That is compatible with every
license below (all permissive, and MPL-2.0/LGPL with their notices kept).

## Currently in the tree (P0-T1)

Verified via `cargo metadata` (backend) and `npm` manifests (frontend). No
copyleft, no special obligations beyond attribution.

| Component | License | Obligation for us |
|---|---|---|
| axum, axum-core, hyper, hyper-util, tower, tower-http, matchit | MIT | Keep copyright notices |
| tokio, mio, socket2, signal-hook-registry | MIT | Keep copyright notices |
| http, http-body, http-body-util, httparse, httpdate, mime, bytes | MIT / MIT OR Apache-2.0 | Keep copyright notices |
| serde, serde_core, serde_json, serde_urlencoded, serde_path_to_error, itoa, ryu | MIT OR Apache-2.0 | Keep copyright notices |
| thiserror, thiserror-impl, syn, quote, proc-macro2 | MIT OR Apache-2.0 | Keep copyright notices |
| tracing, tracing-subscriber, tracing-core, matchers, nu-ansi-term, sharded-slab, thread_local, lazy_static, once_cell | MIT | Keep copyright notices |
| regex-automata, regex-syntax, aho-corasick, memchr | Unlicense OR MIT (we take MIT) | Keep copyright notices |
| libc, cfg-if, pin-project-lite, smallvec, slab, futures-*, pin-project-lite | MIT OR Apache-2.0 | Keep copyright notices |
| frontend: vite, vitest, eslint, typescript-eslint, prettier, typescript, @types/* | MIT / Apache-2.0 | Keep notices |

None of these are GPL-incompatible. No AGPL, no SSPL, no Commons Clause.

## Planned — obligations trigger on the task that adds them

### symphonia (decode/encode) — **MPL-2.0**
File-level weak copyleft (arch §7).

- **Allowed:** yes. MPL-2.0 is one-way-compatible with GPL-3.0.
- **Obligations:**
  - Keep symphonia's copyright + license notices.
  - Any **file** we modify *inside* symphonia must be published under
    MPL-2.0. Do not modify its sources — call its public API only. If a fix
    is genuinely needed, upstream it; if we must patch, publish the patched
    files under MPL-2.0.
  - Merely linking/calling it does **not** make our GPL code MPL-covered.
- **Action when adding (P1-T1 decode, P2-T1 MP3 encode):** add a row above,
  note that we ship an unmodified build, and confirm the LICENSE/NOTICE files
  from the crate are preserved in any bundled distribution.

### lame / libmp3lame (`lame-sys` or CLI) — **LGPL-2.1-or-later**
(arch §7; MP3 export lands in P2-T1.)

- **Preferred:** **dynamic linking** to a system `libmp3lame` so the LGPL
  library stays replaceable. That keeps our obligation to: (a) ship the LGPL
  notice, (b) allow relinking against a modified libmp3lame, (c) not add
  DRM/"effective technological measures" restrictions that stop relinking.
- **Static linking** is also LGPL-compliant but requires shipping the library's
  object files (or a written offer) so a user can relink — more packaging
  burden, so prefer dynamic.
- **Note (D21 / arch §11):** v1 is **source-build only**. We do not ship
  binaries, which removes the "provide the library/notice with the binary"
  packaging work for now. These obligations become real only if we ever ship
  a release bundle (post-v1).
- **Action when adding:** record whether we link dynamically or via the CLI,
  and add the LGPL notice + relink path to the build docs.

## Checklist for the reviewer

- [ ] New dependency's SPDX license recorded in the table above.
- [ ] No AGPL/SSPL/copyleft-strong license added.
- [ ] If MPL-2.0: we call the API, do not modify its files.
- [ ] If LGPL: linking strategy (dynamic preferred) + notice documented.
- [ ] `LICENSE` (GPL-3.0) and per-crate notices preserved in any bundle.
