# BRIA RMBG 1.4 plugin verification — 2026-10-16

The installable artifact is `dist/PhotoCraft-BRIA-RMBG.pcplugin` (169,521,099 bytes;
about 162 MiB). SHA256:
`9ae7aee8984fe9a1809008a28eb09255e541208a0e6183b60814b48aeeec7b21`.
The workspace-root copy is byte-identical. This is a Windows x64 local development
package, with the then-current Plugin API, private SDK/API 0.1, package format 2, image payload 1,
and explicitly declared persistent worker protocol 2.

## Passed

- Eight unit tests: two typed panel-handler tests and six native algorithm/protocol
  tests. They cover action grants, observer behavior, state retention, strict options,
  dimensions, RGB plane order/range, BRIA min/max normalization, constant and invalid
  masks, original transparency/color preservation, cancellation and bounded framing.
- Formatting and Clippy with warnings denied for the panel and native worker.
  Windows occasionally reported an incremental-cache hardlink fallback; compilation
  continued using file copies. There were no Clippy code warnings.
- Actual release builds for import-free `wasm32-unknown-unknown` panel and Windows
  x64 native worker. Model, runtime and Microsoft DLL hashes/sizes were verified.
  Licenses for 54 resolved Rust packages were collected from both lockfiles.
- Current `photocraft-plugin project` ownership audit: every local Rust dependency
  resolves inside this independent project; locked offline resolution passed.
- Current `photocraft-plugin check`: archive paths/CRCs, required payload, real WASM
  exports/manifest, command grants, platform/native header and verified worker-2
  declaration passed. See `project-check.json` and `package-check.json`.
- Extracted exact package, launched with a system-only PATH: real inference on the
  NASA/JSC Apollo 11 photograph, all three output modes, dimensions, unchanged RGB,
  source alpha, complementary subject/background masks, resident model reuse,
  cancellation during inference, invalid options, reset/reload, missing-model error,
  foreign handshake, oversized framing and EOF process exit. See `package-smoke.json`.
  Final runs took 1.30 s initially and approximately 1.0–1.1 s with the resident
  model; cancellation acknowledgement took 0.047 s on this machine. These are sample
  observations, not performance guarantees.
- The latest compiled host engine from the `PhotoCraft-2026-10-16-180847` release
  snapshot exercised the exact artifact in an isolated disposable store. It passed
  installation with native consent, restart discovery, contributed-command
  registration, real inference, 16-bit source conversion, source-preserving preview,
  layer Apply, original-stack visibility, Undo/Redo, Discard, new-tab Apply and DPI,
  stale-source refusal, cancellation, and removal after resident jobs. Final runtime
  status was idle with zero consecutive failures and no last error. The external
  acceptance fixture used the compiled public engine API; no editor source was
  changed and no user plugin installation was touched. See `host-acceptance.json`
  and `host-acceptance.log`.
- Independent source export rebuilt outside the original project with formatting,
  tests, Clippy and release packaging, using its own private SDK and copied,
  checksum-verified asset cache. No sibling plugin or editor source dependency was
  used. See `independent-build.json` and `independent-build.log`.

Visual inspection of the real inferred mask and an opaque-background cutout preview
confirmed the astronaut was retained and the lunar scene removed. Test photo
provenance is recorded in `photo-source.json`; it is public-domain NASA/JSC material.

## Scope and remaining limits

The actual native egui panel was not manually clicked or screenshot-tested in a
visible desktop window. Host behavior above was exercised through its real engine
using the exact WASM/native package, rather than through a mocked host. Other OSes,
GPU execution, memory benchmarks at the 32 MP ceiling, automatic idle eviction and
full disable/update/rollback acceptance were not qualified for this plugin release.

The native interface processes a flattened 8-bit RGBA sRGB composite. It does not
provide native high-precision layers or a separate selection mask. This package
requires the 2026-10-16 worker-2-capable host, not merely an older package-2 host.
BRIA's model has a separate non-commercial license; commercial use requires a BRIA
agreement. Its linked license-agreement page returned HTTP 404 when checked.
The official model card is preserved; no commercial rights or public redistribution
approval is claimed. Nothing was published or installed into the user's store.
