#!/usr/bin/env python3
"""Stage a verified, matched shared Cerlus/WAID/runtime release into out/."""
import argparse
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import subprocess

from runtime_artifact import check, check_elf


def validate_records(prefix, records):
    assert records, 'empty release manifest'
    for name, expected in records.items():
        relative = PurePosixPath(name)
        assert not relative.is_absolute() and '..' not in relative.parts, 'unsafe release path'
        path = prefix / relative
        assert path.resolve().is_relative_to(prefix.resolve()), 'release path escapes its root'
        assert not path.is_symlink() and path.is_file(), f'missing regular release component: {name}'
        with path.open('rb') as stream:
            actual = hashlib.file_digest(stream, 'sha256').hexdigest()
        assert re.fullmatch('[0-9a-f]{64}', expected) and actual == expected, f'release hash mismatch: {name}'


def validate(source):
    receipt = json.loads((source / 'deployment.json').read_text())
    assert receipt['schema_version'] == 1 and receipt['status'] == 'passed', 'unqualified build receipt'
    assert receipt['system'] == 'Linux', 'Debian needs a native Linux release'
    abi = receipt['abi']
    assert re.fullmatch('[0-9a-f]{24}', abi), 'invalid ABI identity'
    arch = subprocess.check_output(['dpkg', '--print-architecture'], text=True).strip()
    expected_target = {'arm64': 'aarch64-unknown-linux-gnu', 'amd64': 'x86_64-unknown-linux-gnu'}[arch]
    assert receipt['target'] == expected_target, 'release target does not match the Debian architecture'
    row = receipt['layouts']['shared']
    prefix = source / 'shared'
    validate_records(prefix, row['files'])
    runtime = f'lib/detir/{abi}/libdetir_runtime-{abi}.so.0'
    standard = f'lib/detir/{abi}/libdetir_std-{abi}.so.0'
    assert row['runtime'] == runtime and row['rust_standard_library'] == standard, 'unexpected loader identities'
    for library in [runtime, standard]:
        assert library in row['files'], f'unrecorded library: {library}'
        check_elf(prefix / library, arch, executable=False)
    check(prefix / 'bin/waid-shell', arch, cli=True)
    for product in ['cerlus', 'waid-shell']:
        binary = prefix / 'bin' / product
        assert f'bin/{product}' in row['files'], 'unrecorded host'
        check_elf(binary, arch)
        info = json.loads(subprocess.check_output([str(binary), '--runtime-info'], text=True))
        assert info == {'product': product, 'layout': 'shared', 'runtime_abi': abi,
                        'version': receipt['version']}, 'host/runtime identity mismatch'
        dynamic = subprocess.check_output(['readelf', '-d', str(binary)], text=True)
        dependencies = re.findall(r'\(NEEDED\).*?\[(.*?)\]', dynamic)
        assert Path(runtime).name in dependencies, 'host does not load the common runtime'
        paths = re.findall(r'\((?:RUNPATH|RPATH)\).*?\[(.*?)\]', dynamic)
        assert paths == [f'$ORIGIN/../lib/detir/{abi}'], 'foreign/build-tree loader paths'
    return receipt, prefix


def stage(source, destination):
    receipt, prefix = validate(source.resolve())
    assert destination.resolve() != prefix.resolve(), 'source and destination must differ'
    # Only these owned build products are replaced; unrelated shell/app output stays.
    for relative in ['lib/detir', 'share/doc/detir-runtime-shared/rust']:
        target = destination / relative
        if target.exists():
            shutil.rmtree(target)
        shutil.copytree(prefix / relative, target)
    (destination / 'bin').mkdir(parents=True, exist_ok=True)
    for product in ['cerlus', 'waid-shell']:
        shutil.copy2(prefix / 'bin' / product, destination / 'bin' / product)
    row = receipt['layouts']['shared']
    validate_records(destination, row['files'])
    data = json.dumps(receipt, indent=2, sort_keys=True) + '\n'
    (destination / 'runtime-deployment.json').write_text(data)
    (destination / 'lib/detir' / receipt['abi'] / 'deployment.json').write_text(data)
    (destination / 'runtime-abi.txt').write_text(receipt['abi'] + '\n')
    print('Staged matched shared runtime ' + receipt['abi'])


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, required=True)
    parser.add_argument('--destination', type=Path, required=True)
    args = parser.parse_args()
    stage(args.source, args.destination)
