"""Build independent panel/worker, verify assets, and produce a Windows x64 package."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import zipfile

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / 'scripts'))
from importlib import import_module
assets = import_module('fetch-assets')

def run(*args):
    subprocess.run(args, cwd=ROOT, check=True)

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--offline', action='store_true')
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    if os.name != 'nt':
        raise RuntimeError('Build natively on Windows x64; this release bundles a Windows runtime.')
    assets.verify()
    flags = ['--locked'] + (['--offline'] if args.offline else [])
    for manifest in ['Cargo.toml','native/Cargo.toml']:
        if args.check:
            run('cargo','fmt','--manifest-path',manifest,'--','--check')
            run('cargo','test','--manifest-path',manifest,*flags)
            run('cargo','clippy','--manifest-path',manifest,*flags,'--all-targets','--','-D','warnings')
    run('cargo','build',*flags,'--release','--target','wasm32-unknown-unknown')
    run('cargo','build','--manifest-path','native/Cargo.toml',*flags,'--release')
    run(sys.executable, 'scripts/collect-licenses.py')
    authoring = json.loads((ROOT/'authoring.json').read_text())
    output = ROOT/'dist/PhotoCraft-BRIA-RMBG.pcplugin'
    output.parent.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='stage-',dir=output.parent) as directory:
        stage = Path(directory)
        (stage/'package.json').write_text(json.dumps(authoring['package'],indent=2)+'\n')
        shutil.copy2(ROOT/'target/wasm32-unknown-unknown/release/photocraft_bria_rmbg.wasm',stage/'plugin.pcext')
        shutil.copy2(ROOT/'native/target/release/bria-rmbg-worker.exe',stage/'bria-rmbg-worker.exe')
        # MinGW dependencies are resolved with only packaged/system DLLs during smoke tests.
        shutil.copytree(ROOT/'runtime',stage/'runtime')
        shutil.copytree(ROOT/'licenses',stage/'licenses')
        for name in ['LICENSE','LICENSE-MIT','LICENSE-APACHE','THIRD-PARTY-NOTICES.md','README.md']:
            shutil.copy2(ROOT/name,stage/name)
        for name in ['LICENSE-MIT','LICENSE-APACHE','SDK-RELEASE.json']:
            path=ROOT/'vendor'/name
            if path.exists(): shutil.copy2(path,stage/'licenses'/('PhotoCraft-SDK-'+name))
        temporary = output.with_suffix('.partial')
        with zipfile.ZipFile(temporary,'w',compression=zipfile.ZIP_DEFLATED,compresslevel=6) as archive:
            for path in sorted(stage.rglob('*')):
                if path.is_file():
                    info=zipfile.ZipInfo(path.relative_to(stage).as_posix(),date_time=(2026,1,1,0,0,0))
                    info.compress_type=zipfile.ZIP_DEFLATED
                    archive.writestr(info,path.read_bytes())
        temporary.replace(output)
    print(f'Installable package: {output}')

if __name__ == '__main__':
    main()
