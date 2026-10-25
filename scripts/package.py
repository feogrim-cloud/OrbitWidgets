"""Package the staged application and selected public sources, without profiles."""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import zipfile


SOURCE_FILES = (
    '.gitignore', '.gitattributes', 'Cargo.toml', 'Cargo.lock', 'LICENSE',
    'README.md', 'README.ru.md', 'README-RU.txt', 'CHANGELOG.md',
    'CONTRIBUTING.md', 'build.ps1',
)
SOURCE_DIRS = ('.cargo', '.github', 'src', 'sensor-helper', 'build-assets',
               'installer', 'scripts', 'docs', 'assets', 'examples')


def public_source_files(root: Path):
    for name in SOURCE_FILES:
        path = root / name
        if path.is_file():
            yield path
    for directory in SOURCE_DIRS:
        for path in sorted((root / directory).rglob('*')):
            if not path.is_file():
                continue
            relative = path.relative_to(root)
            if any(part in ('.source', '__pycache__') for part in relative.parts):
                continue
            if path.name == 'cargo-metadata.json' or path.suffix in ('.pdb', '.pyc', '.log'):
                continue
            if relative.as_posix() == 'build-assets/sensors/OrbitSensors.exe':
                continue
            if directory == 'sensor-helper' and path.suffix == '.exe':
                continue
            yield path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output-dir', type=Path, default=Path('release'))
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    staged = root / 'dist'
    output = args.output_dir.resolve()
    output.mkdir(parents=True, exist_ok=True)
    for required in ('OrbitWidgets.exe', 'sensors/OrbitSensors.exe',
                     'sensors/LibreHardwareMonitorLib.dll', 'optional/PawnIO-Setup.exe'):
        if not (staged / required).is_file():
            raise SystemExit(f'Missing staged build input: {required}; run build.ps1 first')
    installer = output / 'OrbitWidgets-Setup-x64.exe'
    if not installer.is_file():
        raise SystemExit('Build OrbitWidgets-Setup-x64.exe into the output directory first')
    portable = output / 'OrbitWidgets-Windows-x64.zip'
    source = output / 'OrbitWidgets-source.zip'
    with zipfile.ZipFile(portable, 'w', zipfile.ZIP_DEFLATED,
                         compresslevel=7, strict_timestamps=False) as archive:
        for path in sorted(staged.rglob('*')):
            if path.is_file() and '.source' not in path.parts and path.suffix != '.log':
                archive.write(path, Path('OrbitWidgets') / path.relative_to(staged))
    with zipfile.ZipFile(source, 'w', zipfile.ZIP_DEFLATED,
                         compresslevel=7, strict_timestamps=False) as archive:
        for path in public_source_files(root):
            archive.write(path, Path('OrbitWidgets-source') / path.relative_to(root))
    assets = (installer, portable, source)
    checksums = ''.join(f'{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}\n'
                        for path in assets)
    (output / 'SHA256SUMS.txt').write_text(checksums, encoding='utf-8')
    for path in assets:
        print(f'{path.name}: {path.stat().st_size:,} bytes')


if __name__ == '__main__':
    main()
