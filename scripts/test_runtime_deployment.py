"""Refuse corrupt/foreign release components before executing any host."""
from pathlib import Path
import tempfile
import unittest

from stage_runtime_deployment import validate_records


class ReleaseStaging(unittest.TestCase):
    def test_an_empty_manifest_cannot_qualify_a_runtime(self):
        with self.assertRaisesRegex(AssertionError, 'empty'):
            validate_records(Path('/unused'), {})

    def test_release_paths_cannot_escape_the_bundle(self):
        for name in ['../foreign.so', '/usr/lib/foreign.so']:
            with self.subTest(name=name), self.assertRaisesRegex(AssertionError, 'unsafe'):
                validate_records(Path('/unused'), {name: '0' * 64})

    def test_a_modified_component_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'runtime.so').write_bytes(b'a different release')
            with self.assertRaisesRegex(AssertionError, 'hash mismatch'):
                validate_records(root, {'runtime.so': '0' * 64})

    def test_a_symlink_to_a_foreign_library_is_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / 'runtime.so').symlink_to('/usr/lib/foreign.so')
            with self.assertRaisesRegex(AssertionError, 'escapes'):
                validate_records(root, {'runtime.so': '0' * 64})


if __name__ == '__main__':
    unittest.main()
