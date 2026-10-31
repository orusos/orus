/** Bluetooth through `bluetoothctl`, run with the runtime's `host.shell` capability. */

import { exec, run } from "./shell.ts"

export interface Controller {
  powered: boolean
  discovering: boolean
}

export interface BtDevice {
  address: string
  name: string
  paired: boolean
  connected: boolean
  /** BlueZ's icon name, such as `input-mouse` or `audio-headset`. */
  icon: string
}

const MAC = /^[0-9A-F]{2}(:[0-9A-F]{2}){5}$/i

/** `bluetoothctl show`: `null` when this computer has no Bluetooth adapter. */
export function parseController(stdout: string): Controller | null {
  if (!/^Controller /m.test(stdout)) return null
  const field = (name: string) => new RegExp(`^\\s*${name}: (\\w+)`, "m").exec(stdout)?.[1]
  return { powered: field("Powered") === "yes", discovering: field("Discovering") === "yes" }
}

/** `bluetoothctl devices`: one `Device <address> <name>` line each. */
export function parseDeviceList(stdout: string): { address: string; name: string }[] {
  const out: { address: string; name: string }[] = []
  for (const line of stdout.split("\n")) {
    const m = /^Device (\S+) (.*)$/.exec(line.trim())
    if (m && MAC.test(m[1])) out.push({ address: m[1], name: m[2] })
  }
  return out
}

/** `bluetoothctl info <address>`. */
export function parseInfo(stdout: string): Pick<BtDevice, "paired" | "connected" | "icon"> {
  const field = (name: string) => new RegExp(`^\\s*${name}: (.*)$`, "m").exec(stdout)?.[1]?.trim()
  return {
    paired: field("Paired") === "yes",
    connected: field("Connected") === "yes",
    icon: field("Icon") ?? "",
  }
}

/** BlueZ names a device after its address (`AA-BB-...`) when it advertises no name. */
export function isUnnamed(d: { address: string; name: string }): boolean {
  return d.name === d.address.replaceAll(":", "-")
}

export function displayName(d: { address: string; name: string }): string {
  return isUnnamed(d) ? "Unnamed device" : d.name
}

const ctl = (args: string[], timeoutMs?: number) => run("bluetoothctl", args, timeoutMs)

export async function controller(): Promise<Controller | null> {
  // Without the bluetooth service bluetoothctl waits for it forever; the timeout makes
  // that "no adapter".
  const result = await exec("bluetoothctl", ["--timeout", "3", "show"], 8_000)
  return parseController(result.stdout)
}

export async function listDevices(): Promise<BtDevice[]> {
  const found = parseDeviceList(await ctl(["devices"]))
  return Promise.all(
    found.map(async (d) => ({ ...d, ...parseInfo(await ctl(["info", d.address])) })),
  )
}

/** Turning on first lifts a software block (rfkill), which BlueZ reports as "Blocked". */
export async function setPower(on: boolean): Promise<string> {
  if (on) await exec("rfkill", ["unblock", "bluetooth"])
  return ctl(["power", on ? "on" : "off"])
}

/** Look for devices for `seconds`. */
export function scan(seconds: number): Promise<string> {
  return ctl(["--timeout", String(seconds), "scan", "on"], (seconds + 5) * 1000)
}

/**
 * Pair, trust and connect. This pairs devices that need no confirmation (most mice,
 * keyboards and headsets); one that asks for a PIN or a passkey cannot be paired here.
 */
export async function pair(address: string): Promise<void> {
  await ctl(["pair", address], 40_000)
  await ctl(["trust", address])
  await ctl(["connect", address], 20_000)
}

export const connect = (address: string) => ctl(["connect", address], 20_000)
export const disconnect = (address: string) => ctl(["disconnect", address])
export const forget = (address: string) => ctl(["remove", address])
