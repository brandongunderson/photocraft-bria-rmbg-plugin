"""Restore checksum-pinned BRIA and ONNX assets. No third-party Python packages."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]

def digest(path):
    with path.open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()

def verify():
    for name in ['bria-assets.json', 'onnx-assets.json', 'vc-runtime.json']:
        for asset in json.loads((ROOT / 'runtime' / name).read_text())['files']:
            path = ROOT / 'runtime' / asset['path']
            if not path.is_file() or path.stat().st_size != asset['size'] or digest(path) != asset['sha256']:
                raise RuntimeError(f'Missing or invalid pinned runtime asset: {asset["path"]}')

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--offline', action='store_true')
    args = parser.parse_args()
    archives = {}
    for name in ['bria-assets.json', 'onnx-assets.json']:
        manifest = json.loads((ROOT / 'runtime' / name).read_text())
        for asset in manifest['files']:
            path = ROOT / 'runtime' / asset['path']
            if path.is_file() and digest(path) == asset['sha256']:
                continue
            if args.offline:
                raise RuntimeError(f'Missing cached asset: {path}')
            if asset['url'] in archives:
                data = archives[asset['url']]
            else:
                with urllib.request.urlopen(asset['url'], timeout=180) as response:
                    data = response.read()
            if 'archivePath' in asset:
                if hashlib.sha256(data).hexdigest() != manifest['archiveSha256']:
                    raise RuntimeError('ONNX archive checksum mismatch')
                archives[asset['url']] = data
                with zipfile.ZipFile(io.BytesIO(data)) as archive:
                    data = archive.read(asset['archivePath'])
            if len(data) != asset['size'] or hashlib.sha256(data).hexdigest() != asset['sha256']:
                raise RuntimeError(f'Asset checksum mismatch: {asset["path"]}')
            path.parent.mkdir(parents=True, exist_ok=True)
            temporary = path.with_suffix(path.suffix + '.partial')
            temporary.write_bytes(data)
            temporary.replace(path)
    print('BRIA and ONNX assets verified. Restore Microsoft runtime with scripts/fetch-vc-runtime.ps1 if needed.')

if __name__ == '__main__':
    main()
