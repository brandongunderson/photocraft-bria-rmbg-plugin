> Current panel API 2 migration and package identity: [API2-MIGRATION.md](API2-MIGRATION.md). The results below identify their earlier validation artifact.

# BRIA Background Removal 1.0.2 — verification, 2026-10-16

The plugin now opens its panel directly from Plugins > BRIA Background Removal.
The separate quick-removal command contribution was removed from authoring.json.
The matching host menu rule uses a single item when there are no contributed
commands, so there is no Open panel / Remove background submenu.

Panel API 2, SDK/API 0.1, package 2, worker 2, identity and settings are retained.
The native worker remains component version 1.0.1, including its long-path fix.
The release WASM and package were rebuilt from the updated authoring input.

Checks passed: two existing panel tests, Clippy with warnings denied, release
builds, pinned asset verification, locked dependency ownership audit, and actual
WASM/package validation. The native executable matches the verified 1.0.1 package apart from its PE
link timestamp and checksum; every other byte matches. The compiled descriptor has zero contributed commands.
See package-check-1.0.2.json, project-check-1.0.2.json, menu-1.0.2.json and artifacts.json.

Visible desktop interaction was not repeated for this metadata-only change.
The 1.0.1 runtime/long-path acceptance remains historical in TEST-RESULTS-1.0.1.md.
No user installation or editor-core source was changed. Install the updated package
through Manage Plugins with Replace existing plugin. Windows x64 is the qualified
platform; the BRIA model's separate non-commercial license still applies.
