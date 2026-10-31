"""Package boundary tests; synthetic ELF headers are not runtime qualification."""
import io
from pathlib import Path
import struct
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

from runtime_artifact import check_cli, check_elf, extract

HELP = 'Detir WAID app runtime\n\nUsage: waid-shell [OPTIONS] [PATH]\n--dev-url --manifest --app-root\n'


class RuntimeArtifact(unittest.TestCase):
    def setUp(self):
        self.work = tempfile.TemporaryDirectory()
        self.addCleanup(self.work.cleanup)
        self.root = Path(self.work.name)

    def elf(self, machine=183):
        return b'\x7fELF\x02\x01\x01' + bytes(9) + struct.pack('<HH', 3, machine)

    def binary(self, data):
        p = self.root / 'waid-shell'
        p.write_bytes(data)
        p.chmod(0o755)
        return p

    def archive(self, names, *, link=False):
        p = self.root / 'runtime.tgz'
        with tarfile.open(p, 'w:gz') as archive:
            for name in names:
                member = tarfile.TarInfo(name)
                data = self.elf()
                member.size = len(data)
                if link:
                    member.type = tarfile.SYMTYPE
                    member.linkname = '/usr/bin/true'
                archive.addfile(member, None if link else io.BytesIO(data))
        return p

    def test_both_supported_linux_architectures(self):
        check_elf(self.binary(self.elf()), 'arm64')
        check_elf(self.binary(self.elf(62)), 'x64')

    def test_host_script_and_wrong_architecture_are_rejected(self):
        for data in [b'#!/bin/sh\nexit 0\n', b'\xcf\xfa\xed\xfe' + bytes(20), self.elf(62)]:
            with self.assertRaises(ValueError):
                check_elf(self.binary(data), 'arm64')

    def test_shared_library_does_not_require_an_executable_file_mode(self):
        p = self.binary(self.elf())
        p.chmod(0o644)
        check_elf(p, 'arm64', executable=False)

    def test_nonexecutable_binary_is_rejected(self):
        p = self.binary(self.elf())
        p.chmod(0o644)
        with self.assertRaises(ValueError):
            check_elf(p, 'arm64')

    def test_cli_identity_rejects_a_renamed_legacy_browser(self):
        for help_text in ['Usage: detir-scene [COMMAND]\nwaid PATH', HELP.replace('Usage: waid-shell ', 'Usage: waid-shell-old '), HELP.replace('--dev-url', '')]:
            with patch('runtime_artifact.subprocess.run', return_value=subprocess.CompletedProcess([], 0, help_text, '')):
                with self.assertRaises(ValueError):
                    check_cli(self.root / 'waid-shell')
        with patch('runtime_artifact.subprocess.run', return_value=subprocess.CompletedProcess([], 0, HELP, '')):
            check_cli(self.root / 'waid-shell')

    def test_cli_failures_are_not_accepted(self):
        with patch('runtime_artifact.subprocess.run', return_value=subprocess.CompletedProcess([], 1, HELP, 'failed')):
            with self.assertRaises(ValueError):
                check_cli(self.root / 'waid-shell')

    def test_only_the_canonical_regular_archive_member_is_accepted(self):
        destination = self.root / 'out/waid-shell'
        for names in [['detir-scene'], ['../waid-shell'], ['waid-shell', 'waid-shell'], ['waid-shell', 'extra']]:
            with self.assertRaises(ValueError):
                extract(self.archive(names), destination, 'arm64')
            self.assertFalse(destination.exists())
        with self.assertRaises(ValueError):
            extract(self.archive(['waid-shell'], link=True), destination, 'arm64')

    def test_extract_installs_a_validated_executable(self):
        destination = self.root / 'out/waid-shell'
        extract(self.archive(['waid-shell']), destination, 'arm64')
        self.assertEqual(destination.read_bytes(), self.elf())
        self.assertEqual(destination.stat().st_mode & 0o777, 0o755)

    def test_failed_validation_preserves_previous_staged_binary(self):
        destination = self.root / 'out/waid-shell'
        destination.parent.mkdir()
        destination.write_bytes(b'previous artifact')
        with self.assertRaises(ValueError):
            extract(self.archive(['waid-shell']), destination, 'x64')
        self.assertEqual(destination.read_bytes(), b'previous artifact')


if __name__ == '__main__':
    unittest.main()
