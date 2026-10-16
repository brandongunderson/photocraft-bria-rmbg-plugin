# BRIA Background Removal changelog

Release history for this standalone PhotoCraft plugin. The first published GitHub release
is 1.0.2; it includes the earlier 1.0.1 native long-path fix.

## [1.0.2](https://github.com/brandongunderson/photocraft-bria-rmbg-plugin/releases/tag/v1.0.2) - 2026-10-16

### Changed

- Open the panel from **Plugins > BRIA Background Removal**. Remove the redundant quick
  action and its submenu.

### Fixes included

- Retain the native worker 1.0.1 fix for model and ONNX Runtime loading in Windows
  installation paths longer than the traditional 260-character boundary.

### Included

- Local CPU background removal with the official BRIA RMBG 1.4 model and a private Windows
  ONNX Runtime installation.
- Foreground, background and mask results through PhotoCraft's shared comparison preview,
  Apply/Discard and Undo workflow.
- A private persistent worker using explicit protocol 2, with retained model state, progress,
  cancellation, reset and idle cleanup.
- A typed panel, private SDK/API 0.1, one authoring manifest and independent source builds.

### Compatibility and license

Requires Windows x64, the current panel contract (internal discriminator `2`), package
format 2, `host.commands.invoke` version 1 and persistent native worker protocol 2. The
native worker component remains version 1.0.1. A discrete GPU is not required.

The BRIA model permits non-commercial use; commercial use requires a separate BRIA
agreement. Preserve the model terms, runtime notices and provenance when redistributing.

See [migration verification](verification/API2-MIGRATION.md),
[menu-change verification](verification/TEST-RESULTS.md) and
[long-path verification](verification/TEST-RESULTS-1.0.1.md) for the recorded checks and
acceptance limits. [README.md](README.md) contains installation and resource limits.
