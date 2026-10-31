"""Preflight refusal tests do not substitute for booted-image evidence."""
import unittest
import json
from pathlib import Path
import tempfile
from check_runtime_image import check_image_environment, read_observation


class ImagePreflight(unittest.TestCase):
    def check(self, boot='boot=live components', release='PRETTY_NAME="Orus OS"', env=None):
        check_image_environment(boot, release, {'WAYLAND_DISPLAY': 'wayland-0'} if env is None else env)

    def test_normal_live_session_can_reach_the_hardware_checks(self):
        self.check()

    def test_builder_or_installed_session_is_not_live_image_proof(self):
        with self.assertRaises(AssertionError):
            self.check(boot='root=/dev/vda3')

    def test_old_or_unbranded_image_is_refused(self):
        with self.assertRaises(AssertionError):
            self.check(release='PRETTY_NAME="Debian GNU/Linux 13"')

    def test_no_wayland_session_is_refused(self):
        with self.assertRaises(AssertionError):
            self.check(env={})

    def test_development_overrides_cannot_qualify_production_startup(self):
        for name in ['DETIR_WAID_DEV_SECRETS_FILE', 'DETIR_WAID_DEV_URL',
                     'DETIR_WAID_DEV_MANIFEST', 'DETIR_WAID_DEV_APP_ROOT']:
            with self.subTest(name=name), self.assertRaises(AssertionError):
                self.check(env={'WAYLAND_DISPLAY': 'wayland-0', name: 'configured'})

    def trace(self, observations):
        folder = tempfile.TemporaryDirectory()
        self.addCleanup(folder.cleanup)
        path = Path(folder.name) / 'trace.jsonl'
        path.write_text('\n'.join(json.dumps({'ev': 'eval', 'result': json.dumps(row)})
                                  for row in observations))
        return path

    def test_click_proof_is_read_from_runtime_output(self):
        observed = {'proof': 'SS-T14', 'clicks': 1, 'title': 'SS-T14 Runtime Check'}
        self.assertEqual(read_observation(self.trace([observed])), observed)

    def test_missing_or_duplicate_observations_fail(self):
        observed = {'proof': 'SS-T14', 'clicks': 1, 'title': 'SS-T14 Runtime Check'}
        for rows in [[], [observed, observed]]:
            with self.assertRaises(AssertionError):
                read_observation(self.trace(rows))

    def test_zero_or_invalid_click_observations_fail(self):
        for clicks in [0, 2, True, '1']:
            with self.assertRaises(AssertionError):
                read_observation(self.trace([{'proof': 'SS-T14', 'clicks': clicks,
                                              'title': 'SS-T14 Runtime Check'}]))


if __name__ == '__main__':
    unittest.main()
