# Third-party notices

BRIA RMBG 1.4 is developed by BRIA AI. This independent PhotoCraft plugin is not
endorsed by BRIA. The model has its own **bria-rmbg-1.4 non-commercial license**;
the plugin source license does not relicense the model. Commercial use requires a
separate agreement with BRIA. See https://huggingface.co/briaai/RMBG-1.4 and the
unmodified model card in `runtime/licenses/BRIA-MODEL-CARD.md`. The model card's
linked license-agreement page returned HTTP 404 when checked on 2026-10-16;
consult BRIA for the complete agreement before commercial use or redistribution.
This local development package is not evidence of commercial licensing rights.

Model provenance, exact upstream revision, download URL, size and SHA256 are
recorded in `runtime/bria-assets.json`. The official float32 ONNX graph is used
without modification. Preprocessing and normalization follow BRIA's published
model-card example. No remote model code is executed.

Microsoft ONNX Runtime 1.22.0 is MIT licensed. Its license and third-party notices
are in `runtime/licenses/`. The app-local Microsoft Visual C++ runtime retains
its separate redistribution license in that same folder. Assets and archive
checksums are pinned in `runtime/onnx-assets.json` and `runtime/vc-runtime.json`.

The privately exported PhotoCraft SDK/API source retains its original MIT or
Apache-2.0 licenses and `vendor/SDK-RELEASE.json` provenance. Rust dependency
licenses are collected from the locked resolutions in `licenses/rust-dependencies.txt`.
