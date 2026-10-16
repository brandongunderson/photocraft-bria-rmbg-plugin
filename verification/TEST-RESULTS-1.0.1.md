# BRIA Background Removal 1.0.1 — verification, 2026-10-16

This release fixes native asset loading from the Windows portable installation
reported by the user. Panel API 2, private SDK/API 0.1, package 2 and persistent
worker protocol 2 are retained. There are no new dependencies or editor-core changes.
Exact current package/source identities are recorded in `artifacts.json`.

## Root cause and fix

The user's installed `runtime/bin/onnxruntime.dll` path was 262 characters long.
The file and its bundled VC libraries were present, but the Windows loader rejected
the ordinary path beyond its traditional MAX_PATH boundary. A direct load of that
same file failed with WinError 206; its extended-length path loaded successfully.

The old package was reproduced in a private extracted installation with the same
262-character DLL path and returned the exact reported error. See
`long-path-before.log` and `long-path-before-diagnostics.log`. The old generic error
incorrectly suggested a Windows-version problem on this Windows 11 machine.

The worker now canonicalizes both the runtime library and model file before passing
them to third-party native APIs. On Windows this produces normalized extended-length
paths, including the proper UNC representation. Runtime initialization failures now
retain the bounded underlying native-loader diagnostic.

## Passed for the fixed package

- Two typed panel tests and six native algorithm/protocol tests, formatting, Clippy
  with warnings denied, and release WASM/native builds. See `build-1.0.1.log`.
- Pinned model/runtime/VC asset hashes and sizes, with 54 resolved Rust package
  licenses collected. Both project lockfiles retain the dependency resolutions.
- Current conformance-tool package validation: archive/CRCs, target, native header,
  actual WASM ABI/descriptor, grants, and verified worker-2 declaration. See
  `package-check-1.0.1.json`.
- The exact final package, extracted to the previously failing 262-character DLL
  path: all three output modes, original RGB/transparency, complementary masks,
  resident reuse, cancellation, reset/reload, invalid options, missing-model error,
  foreign handshake, oversized frames and EOF. The model path also crosses the
  traditional boundary. See `package-long-path-smoke.json` and `long-path-after.log`.
- Actual host acceptance from a disposable installed store with the same DLL-path
  length: install/restart, contributed-command registration, inference, 16-bit source
  conversion, source-preserving preview, layer Apply and original-stack visibility,
  Undo/Redo, Discard, new-tab Apply/DPI, stale-source rejection, cancellation and
  removal. Runtime ended idle, with zero consecutive failures and no last error.
  The receipt records the exact package SHA256. See
  `host-matching-path-1.0.1.json` and its matching log.
- Locked offline project dependency ownership audit. All local dependencies remain
  private to this source project. See `project-check-1.0.1.json`.

The original 1.0.0 acceptance record and independent clean source-build evidence are
historical; see `TEST-RESULTS-1.0.0.md` and `independent-build.json`. The 1.0.1 native
and WASM components were rebuilt locally, and its source ZIP was refreshed.

## Scope

No user installation, lock file, activation record, or running editor was changed.
Install 1.0.1 through Manage Plugins using Replace existing plugin. Its submenu is
unchanged: the registered panel and separate quick command remain two entry points.

A deliberately deeper test with a 300-character DLL path exposed a separate host
launch limitation when the worker's working directory itself exceeds MAX_PATH:
Windows rejected the launch with error 267 before the worker ran. That editor-host
limitation is not changed by this plugin patch; see `host-long-path-1.0.1.log`.
The user's 262-character installation and matching installed-store test pass.

Mac/Linux, GPU inference and manual visible-desktop panel interaction are not
qualified here. The model still has BRIA's separate non-commercial license;
commercial use requires a separate agreement. Nothing was published.
