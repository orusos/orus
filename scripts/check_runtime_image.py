#!/usr/bin/env python3
"""Run inside a booted Orus desktop live image, in its normal labwc user session."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import time

from runtime_artifact import check


def sha(path):
    with Path(path).open('rb') as stream:
        return hashlib.file_digest(stream, 'sha256').hexdigest()


def command(*args):
    return subprocess.check_output(args, text=True, timeout=15).strip()


def check_image_environment(boot, release, env):
    assert 'boot=live' in boot, 'live-image boot proof is required'
    assert 'PRETTY_NAME="Orus OS"' in release, 'boot the rebuilt Orus image'
    assert env.get('WAYLAND_DISPLAY'), 'run in the normal Wayland user session'
    for variable in ['DETIR_WAID_DEV_SECRETS_FILE', 'DETIR_WAID_DEV_URL',
                     'DETIR_WAID_DEV_MANIFEST', 'DETIR_WAID_DEV_APP_ROOT']:
        assert not env.get(variable), f'{variable} must not bypass normal package startup'


def read_observation(trace):
    observations = []
    for line in trace.read_text().splitlines():
        row = json.loads(line)
        if row.get('ev') == 'eval':
            value = json.loads(row['result'])
            if isinstance(value, dict) and value.get('proof') == 'SS-T14':
                observations.append(value)
    assert len(observations) == 1, 'missing or duplicated runtime observation'
    observed = observations[0]
    assert type(observed.get('clicks')) is int and observed['clicks'] == 1, 'native click did not reach the app'
    assert observed.get('title') == 'SS-T14 Runtime Check', 'unexpected page'
    return observed


def run(out):
    runtime = Path('/usr/bin/waid-shell')
    assert os.uname().sysname == 'Linux', 'run inside the booted Linux image'
    check_image_environment(Path('/proc/cmdline').read_text(),
                            Path('/etc/os-release').read_text(), os.environ)
    for tool in ['dpkg-query', 'wlrctl', 'grim', 'pgrep']:
        assert shutil.which(tool), f'{tool} is required'
    arch = command('dpkg', '--print-architecture')
    check(runtime, arch, cli=True)
    owner = command('dpkg-query', '-S', str(runtime))
    assert owner.split(':', 1)[0] == 'waid-runtime', 'runtime must belong to the installed package'
    compositor = command('pgrep', '-u', str(os.getuid()), '-x', 'labwc').splitlines()
    app = out / 'app'
    app.mkdir()
    manifest = {'appUniversalName': 'ss-t14-runtime-check', 'nativeUrlScheme': 'dev.orus.sst14',
                'productName': 'SS-T14 Runtime Check', 'version': '1.0.0', 'capabilities': []}
    (app / 'waid-package.json').write_text(json.dumps(manifest) + '\n')
    (app / 'index.html').write_text('''<!doctype html><html><head><meta charset="utf-8">
<title>SS-T14 Runtime Check</title><style>
body{margin:24px;background:#101827;color:white;font:20px sans-serif}
#status{position:absolute;left:24px;top:240px}#frames{position:absolute;left:24px;top:300px}
button{position:absolute;left:24px;top:160px;width:360px;height:64px;background:#22c99a;border:0;font:20px sans-serif}
</style></head><body><h1>SS-T14 app launch</h1>
<p>Standalone waid-shell · Debian package · labwc</p><p id="status">Clicks: 0</p>
<button id="go">Test native input</button><small id="frames"></small><script>
globalThis.ssT14Clicks=0;globalThis.ssT14Ready=true;globalThis.ssT14Frames=0;
document.getElementById('go').addEventListener('click',()=>{ssT14Clicks++;document.getElementById('status').textContent='Clicks: '+ssT14Clicks;});
requestAnimationFrame(function tick(){document.getElementById('frames').textContent='Frame '+(++ssT14Frames);requestAnimationFrame(tick);});
</script></body></html>''')
    steps = [{'wait_for_js': 'globalThis.ssT14Ready===true', 'timeout_ms': 20000},
             {'wait_for_js': 'globalThis.ssT14Frames>=10', 'timeout_ms': 20000},
             {'cursor': [180, 192]}, {'mouse_left': 'down'}, {'mouse_left': 'up'},
             {'wait_for_js': 'globalThis.ssT14Clicks===1', 'timeout_ms': 5000},
             {'eval_js': 'JSON.stringify({proof:"SS-T14",clicks:ssT14Clicks,title:document.title})'},
             {'wait_for_js': 'globalThis.ssT14Frames>=60', 'timeout_ms': 20000},
             {'wait_ms': 1000}, {'exit': True}]
    script = out / 'steps.json'
    script.write_text(json.dumps(steps, indent=2) + '\n')
    frame = out / 'runtime.png'
    env = os.environ | {'DETIR_BENCH_SCRIPT': str(script), 'DETIR_SNAPSHOT_OUT': str(frame),
                        'DETIR_PERF_TRACE_OUT': str(out / 'runtime.jsonl'),
                        'DETIR_SNAPSHOT_AFTER_FRAMES': '30',
                        'DETIR_WINDOW_WIDTH': '800', 'DETIR_WINDOW_HEIGHT': '600'}
    env.pop('DETIR_EXIT_AFTER_SNAPSHOT', None)
    start = time.monotonic()
    captured = False
    proc_exe = None
    with (out / 'runtime.log').open('w') as log:
        process = subprocess.Popen([str(runtime), str(app)], env=env, stdout=log, stderr=subprocess.STDOUT)
        try:
            while process.poll() is None:
                if proc_exe is None:
                    proc_exe = os.readlink(f'/proc/{process.pid}/exe')
                assert time.monotonic() - start < 45, 'runtime launch timed out'
                if frame.is_file() and not captured:
                    windows = command('wlrctl', 'toplevel', 'list')
                    if 'SS-T14 Runtime Check' in windows:
                        (out / 'windows.txt').write_text(windows + '\n')
                        subprocess.run(['grim', str(out / 'desktop.png')], check=True, timeout=15)
                        captured = True
                time.sleep(0.1)
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()
    assert process.returncode == 0, 'runtime failed; see runtime.log'
    assert Path(proc_exe).resolve() == runtime.resolve(), 'unexpected executed program'
    assert '[bench] script complete' in (out / 'runtime.log').read_text(), 'native input script did not complete'
    assert captured, 'no matching labwc window and desktop capture'
    observed = read_observation(out / 'runtime.jsonl')
    header = frame.read_bytes()[:24]
    assert header[:8] == b'\x89PNG\r\n\x1a\n', 'runtime screenshot missing'
    dimensions = struct.unpack('>II', header[16:24])
    assert dimensions[0] >= 600 and dimensions[1] >= 400, 'unexpected content viewport'
    return {'status': 'passed', 'runtime': str(runtime), 'runtime_sha256': sha(runtime),
            'package': owner, 'version': command('dpkg-query', '-W', '-f=${Version}', 'waid-runtime'),
            'architecture': arch, 'executed_program': proc_exe, 'compositor_pids': compositor,
            'boot_cmdline': Path('/proc/cmdline').read_text().strip(),
            'os_release': Path('/etc/os-release').read_text(), 'native_clicks': observed['clicks'],
            'observation': observed,
            'screenshot_dimensions': dimensions, 'runtime_frame_sha256': sha(frame),
            'desktop_frame_sha256': sha(out / 'desktop.png'),
            'fixture_sha256': sha(app / 'index.html'), 'manifest_sha256': sha(app / 'waid-package.json'),
            'scope': 'One standalone Debian WAID app under labwc; no full browser/GPU/platform qualification'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    args.out = args.out.resolve()
    args.out.mkdir(parents=True, exist_ok=False)
    try:
        result = run(args.out)
    except (AssertionError, OSError, ValueError, KeyError, struct.error, subprocess.SubprocessError) as error:
        result = {'status': 'failed', 'reason': str(error)}
    (args.out / 'receipt.json').write_text(json.dumps(result, indent=2) + '\n')
    print(result['status'])
    raise SystemExit(0 if result['status'] == 'passed' else 1)
