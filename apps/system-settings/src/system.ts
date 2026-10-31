/**
 * Battery and machine facts, read with fixed shell scripts (no user input reaches them).
 * The base system is read from /etc/debian_version because os-release names Orus OS.
 */

import { run } from "./shell.ts"

export interface BatteryInfo {
  name: string
  percent: number
  status: string
}

export interface AboutInfo {
  hostname: string
  os: string
  kernel: string
  arch: string
  /** Installed memory in GiB, one decimal. */
  memoryGiB: string
  /** Version of the `detir-shell` package, when it is installed from a .deb. */
  version: string
}

const POWER_SCRIPT = `for b in /sys/class/power_supply/BAT*; do
  [ -d "$b" ] || continue
  echo "$(basename "$b")|$(cat "$b/capacity")|$(cat "$b/status")"
done`

const ABOUT_SCRIPT = `echo "host=$(hostname)"
echo "kernel=$(uname -r)"
echo "arch=$(uname -m)"
if [ -r /etc/debian_version ]; then echo "os=Debian GNU/Linux $(cat /etc/debian_version)"; else echo "os=$(. /etc/os-release; echo "$PRETTY_NAME")"; fi
echo "mem=$(awk '/MemTotal/ {print $2}' /proc/meminfo)"
echo "version=$(dpkg-query -W -f '\${Version}' detir-shell 2>/dev/null)"`

export function parsePower(stdout: string): BatteryInfo[] {
  return stdout
    .split("\n")
    .map((line) => line.split("|"))
    .filter((f) => f.length === 3 && f[0] !== "" && /^\d+$/.test(f[1]))
    .map(([name, percent, status]) => ({
      name,
      percent: Math.min(100, Number(percent)),
      status: status.trim(),
    }))
}

export function parseAbout(stdout: string): AboutInfo {
  const values = new Map<string, string>()
  for (const line of stdout.split("\n")) {
    const at = line.indexOf("=")
    if (at > 0) values.set(line.slice(0, at), line.slice(at + 1).trim())
  }
  const kib = Number(values.get("mem"))
  return {
    hostname: values.get("host") ?? "",
    os: values.get("os") ?? "",
    kernel: values.get("kernel") ?? "",
    arch: values.get("arch") ?? "",
    memoryGiB: kib > 0 ? (kib / 1024 / 1024).toFixed(1) : "",
    version: values.get("version") ?? "",
  }
}

export async function readPower(): Promise<BatteryInfo[]> {
  return parsePower(await run("sh", ["-c", POWER_SCRIPT]))
}

export async function readAbout(): Promise<AboutInfo> {
  return parseAbout(await run("sh", ["-c", ABOUT_SCRIPT]))
}
