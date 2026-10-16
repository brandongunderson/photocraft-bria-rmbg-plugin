# Panel API 2 migration — 1.0.2

Verified locally on Windows x64 on October 16, 2026. This record identifies the
current API 2 package; earlier validation records describe their own prior artifacts.

- Package: `PhotoCraft-BRIA-RMBG.pcplugin` (169,522,025 bytes).
- SHA256: `cab87ca7955e65fd87fbf741d4ec9ab73c6a23877d5b384854e69e9eeab3c5df`.
- Panel API 2; private SDK/API 0.1; package format 2.
- Required capability: `host.commands.invoke` version 1.
- Rust tests: 2 panel and 6 native tests passed.
- Both Cargo workspaces: formatting and all-target Clippy passed.
- Release WASM/native package build, archive CRC, conformance and private SDK hashes passed.
- The panel also passed tests and release WASM compilation from a clean source export,
  without downloaded native runtimes/models or an editor/sibling source dependency.
- Current API-2-only host: install, restart discovery, real processing of a 16-bit source,
  source-preserving preview, layer Apply, Undo/Redo, Discard, new-tab Apply/DPI,
  stale-source rejection, cancellation/reaping and removal passed in a disposable store.

BRIA already used API 2. Its private SDK snapshot was refreshed and verified against its release index; persistent worker protocol 2 and the current native loading fix are retained. Cancellation may leave a brief store-guard cleanup interval; the acceptance fixture retries a busy removal for at most one second.

The three migrated panels use the SDK's bounded ABI buffers and typed event/response
contracts. `authoring.json` is the descriptor/package source; generated output is not edited.
Panel versions, authoring format, package versions, host metadata and worker protocols are
separate contracts. Retirement of panel API 1 does not retire worker protocol 1.

These are engine integration checks. Desktop click-through was not repeated. No user
installation was modified during validation. Published packages and source are available in the repository Releases page.
Machine-specific details are in `api2-build-checks.json` and `api2-*.log` beside this file;
workspace acceptance/conformance and clean-source evidence are under `extension-review/`.
