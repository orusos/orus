#!/usr/bin/env python3
"""Validate the Linux WAID CLI/ELF before staging either deployment layout."""
import argparse
import os
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tarfile
import tempfile

MACHINES = {'arm64': 183, 'aarch64': 183, 'x64': 62, 'amd64': 62, 'x86_64': 62}


def check_elf(binary, arch, executable=True):
    if binary.is_symlink() or not binary.is_file():
        raise ValueError('runtime must be a regular executable file')
    with binary.open('rb') as stream:
        header = stream.read(20)
    if len(header) != 20 or header[:7] != b'\x7fELF\x02\x01\x01':
        raise ValueError('runtime must be a 64-bit little-endian Linux ELF, not a host binary or script')
    kind, machine = struct.unpack('<HH', header[16:20])
    if kind not in (2, 3) or machine != MACHINES[arch]:
        raise ValueError(f'runtime ELF architecture does not match {arch}')
    if executable and not os.access(binary, os.X_OK):
        raise ValueError('runtime is not executable')


def check_cli(binary):
    result = subprocess.run([str(binary.resolve()), '--help'], capture_output=True,
                            text=True, timeout=15, check=False)
    help_text = result.stdout + result.stderr
    if result.returncode or not re.search(r'^Usage: waid-shell(?:\s|$)', help_text, re.M):
        raise ValueError('runtime is not the standalone waid-shell CLI; rebuild from the current Detir source')
    if not all(flag in help_text for flag in ('[PATH]', '--dev-url', '--manifest', '--app-root')):
        raise ValueError('runtime lacks the WAID direct-path/development CLI contract')


def check(binary, arch, cli=False):
    check_elf(binary, arch)
    if cli:
        check_cli(binary)


def extract(archive, destination, arch, cli=False):
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=destination.parent) as directory:
        staged = Path(directory) / 'waid-shell'
        with tarfile.open(archive, 'r:gz') as source:
            members = source.getmembers()
            if len(members) != 1 or members[0].name != 'waid-shell' or not members[0].isfile():
                raise ValueError('runtime archive must contain only waid-shell; legacy detir-scene archives cannot be renamed into a product')
            if not 0 < members[0].size <= 1024**3:
                raise ValueError('runtime archive has an invalid executable size')
            with source.extractfile(members[0]) as data, staged.open('wb') as output:
                shutil.copyfileobj(data, output)
        staged.chmod(0o755)
        check(staged, arch, cli)
        os.replace(staged, destination)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--arch', required=True, choices=MACHINES)
    parser.add_argument('--check-cli', action='store_true')
    action = parser.add_mutually_exclusive_group(required=True)
    action.add_argument('--binary', type=Path)
    action.add_argument('--archive', type=Path)
    parser.add_argument('--destination', type=Path)
    args = parser.parse_args()
    if args.archive:
        if args.destination is None:
            parser.error('--archive requires --destination')
        extract(args.archive, args.destination, args.arch, args.check_cli)
    else:
        check(args.binary, args.arch, args.check_cli)


if __name__ == '__main__':
    try:
        main()
    except (ValueError, OSError, tarfile.TarError, subprocess.SubprocessError) as error:
        raise SystemExit(f'waid-runtime: {error}')
