# Standalone plugin project

Read README.md, authoring.json and src/lib.rs. Keep local dependencies inside this root.
Edit authoring.json, not generated Cargo output; preserve the stable plugin identity on updates.
The SDK macro owns bounded ABI buffers. Never add unsafe pointer operations or host imports.
Implement feature logic here and request only required host capabilities. Generated bindings
and discovery do not grant authority. Run native checks, actual release WASM host validation,
and install/invoke/Undo tests in a disposable store. Keep target/, dist/ and generated scratch
files out of source commits. Existing panel/filter/native protocol versions are independent.

This project deliberately targets panel API 2 / private SDK 0.1, package 2, and
worker protocol 2 via runtime/worker-v2.json. Do not fall back to worker 1 or share
another plugin's process or model directory. Keep authoring.json and the native
handshake identity aligned. Native cancellation must terminate inference, finish
job-file access before acknowledgement, and never commit a cancelled result.

Native code, models, checksums, notices and tests belong here. Preserve the BRIA
non-commercial notice. Use build.py --check --offline, tests/smoke_worker.py and
the conformance tool. Record any desktop Apply/Undo acceptance not exercised.

Maintain CHANGELOG.md for this plugin's releases, with version/date, behavior changes and
compatibility notes. Keep shared platform history in the editor repository. Record future
changes under Unreleased until publishing a tag; do not retag a release for documentation edits.
