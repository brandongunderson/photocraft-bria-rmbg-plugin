# PhotoCraft BRIA Background Removal

[Source repository](https://github.com/brandongunderson/photocraft-bria-rmbg-plugin) · [Changelog](CHANGELOG.md)

[Download the latest plugin](https://github.com/brandongunderson/photocraft-bria-rmbg-plugin/releases/latest/download/PhotoCraft-BRIA-RMBG.pcplugin) · [All releases](https://github.com/brandongunderson/photocraft-bria-rmbg-plugin/releases)

Local background removal using the official [BRIA RMBG 1.4](https://huggingface.co/briaai/RMBG-1.4)
float32 ONNX model. This independent plugin uses our current **typed panel API 2 / SDK 0.1**,
**package format 2**, and explicitly declared **persistent native worker protocol 2**.
It owns its model, runtime, worker and private SDK sources. No Python, companion app,
another plugin, editor source checkout, network access or discrete GPU is required at runtime.

## Install and use

Use the PhotoCraft 2026-10-16 test build or a newer build implementing worker protocol 2.
Older package-2 hosts without this transport are incompatible with this release.

Version 1.0.2 opens the panel directly from **Plugins → BRIA Background Removal**.
The redundant quick-action contribution and its submenu have been removed.

Version 1.0.1 fixes ONNX Runtime and model loading when Windows installation paths
exceed the traditional 260-character boundary. Native asset paths are resolved to
Windows' extended-length form before passing them to ONNX. Updates keep the same
plugin identity; review the new package and choose **Update plugin** in the manager.

1. In **Plugins → Manage Plugins → Browse**, choose `PhotoCraft-BRIA-RMBG.pcplugin`.
2. Review the native-code permission and install.
3. Open an image, then choose **Plugins → BRIA Background Removal**.
4. Choose `foreground` to keep the subject, `background` to keep the background, or
   `mask` for an opaque grayscale subject mask. Select **Create result**.
5. Compare the preview, then **Apply** or **Discard**. Apply can open a new tab or
   add a layer at the original dimensions. Layer Apply is undoable.

Requirements: Windows x64, Windows 10 build 19041+ or Windows 11. The package bundles
ONNX Runtime 1.22.0 and its app-local Microsoft VC dependencies. CPU inference can
use several GB of RAM; the worker declares a 3 GiB memory admission estimate, not a
hard memory limit. It uses up to four inference threads. The loaded model is retained
between runs and released when the host resets/stops the worker or after two idle minutes.

## Image behavior

PhotoCraft supplies the visible flattened 8-bit RGBA sRGB composite, including conversion
from supported 8/16-bit RGB/grayscale documents. The result is also 8-bit RGBA sRGB;
the host retains DPI. There is no per-layer, separate selection-mask, or high-precision
native image transport. Maximum input is 32 megapixels and 32,768 pixels per side;
host resource admission or available RAM may impose lower practical limits.

Only the mask is inferred at 1024 × 1024 and restored to the original dimensions.
Source RGB pixels are retained exactly. Subject/background alpha is multiplied by
existing source alpha; transparent pixels are never made opaque. The mask output
includes source coverage. Keeping background does not reconstruct hidden background.
Fine hair, glass and difficult edges can need manual refinement.

Preprocessing follows BRIA's published PyTorch example: bilinear float RGB resize,
RGB / 255 − 0.5, NCHW, then float mask resize followed by min/max normalization.
There is no extra sigmoid. Constant masks retain their clamped model confidence,
avoiding the reference example's division by zero. The official graph is unmodified.

## Build from independent source

Use Rust 1.95+, `wasm32-unknown-unknown`, a Windows x64 linker, Python 3.11+ for
build tooling, and PowerShell for Microsoft runtime extraction. From this directory:

```powershell
python scripts/fetch-assets.py
./scripts/fetch-vc-runtime.ps1
python build.py --check --offline
```

Output: `dist/PhotoCraft-BRIA-RMBG.pcplugin`. Models/binary runtime files and build
outputs are ignored in source control; pin manifests and locks are source files.
`fetch-assets.py --offline` verifies cached assets; the build verifies all checksums
and sizes, collects dependency licenses, builds release WASM/native binaries, and
packages metadata directly from `authoring.json`. No generated manifest needs editing.
Use `photocraft-plugin project .` and `check dist/PhotoCraft-BRIA-RMBG.pcplugin`
when the optional conformance tool is available. Check does not execute native code.

`tests/smoke_worker.py PACKAGE` exercises an extracted package with a system-only PATH,
including real inference, repeat requests, transparency, cancellation, reset, malformed
frames and EOF. Results and remaining acceptance limits are in `verification/TEST-RESULTS.md`.

## Model license

BRIA's model is available for **non-commercial use**; commercial use requires a
separate BRIA agreement. Source code licensing does not grant commercial model rights.
The upstream model card and exact asset provenance are bundled. Its linked license
page returned HTTP 404 on 2026-10-16; consult BRIA before commercial use or redistribution.
See `THIRD-PARTY-NOTICES.md`. This project has not been published or installed into
the user's existing plugin store.
