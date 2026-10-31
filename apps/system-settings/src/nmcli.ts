/** NetworkManager through `nmcli`, run with the runtime's `host.shell` capability. */

import { run } from "./shell.ts"

export interface WifiNetwork {
  ssid: string
  /** 0-100. */
  signal: number
  /** Empty for an open network. */
  security: string
  active: boolean
  /** A saved connection with this name exists, so no password is needed. */
  saved: boolean
}

export interface NetDevice {
  name: string
  type: string
  state: string
  connection: string
  addresses: string[]
}

/** nmcli's `-t` output separates fields with `:` and escapes `:` and `\` with a backslash. */
export function splitTerse(line: string): string[] {
  const fields: string[] = []
  let current = ""
  for (let i = 0; i < line.length; i++) {
    const c = line[i]
    if (c === "\\" && i + 1 < line.length) {
      current += line[++i]
    } else if (c === ":") {
      fields.push(current)
      current = ""
    } else {
      current += c
    }
  }
  fields.push(current)
  return fields
}

function rows(stdout: string): string[][] {
  return stdout
    .split("\n")
    .filter((line) => line.length > 0)
    .map(splitTerse)
}

/** One entry per SSID (the strongest), the connected one first, then by signal. */
export function parseWifiList(stdout: string, savedNames: Set<string>): WifiNetwork[] {
  const best = new Map<string, WifiNetwork>()
  for (const [inUse, ssid, signal, security] of rows(stdout)) {
    if (!ssid) continue
    const network: WifiNetwork = {
      ssid,
      signal: Number(signal) || 0,
      security: security ?? "",
      active: inUse === "*",
      saved: savedNames.has(ssid),
    }
    const seen = best.get(ssid)
    if (!seen || network.active || (!seen.active && network.signal > seen.signal)) {
      best.set(ssid, network)
    }
  }
  return [...best.values()].sort(
    (a, b) => Number(b.active) - Number(a.active) || b.signal - a.signal,
  )
}

export function parseDeviceStatus(stdout: string): Omit<NetDevice, "addresses">[] {
  return rows(stdout)
    .map(([name, type, state, connection]) => ({
      name,
      type,
      state,
      connection: connection ?? "",
    }))
    .filter((d) => d.type === "ethernet" || d.type === "wifi")
}

export function parseAddresses(stdout: string): string[] {
  return rows(stdout)
    .filter(([key]) => key.startsWith("IP4.ADDRESS"))
    .map(([, value]) => value)
}

export function parseSavedWifi(stdout: string): Set<string> {
  return new Set(
    rows(stdout)
      .filter(([, type]) => type === "802-11-wireless")
      .map(([name]) => name),
  )
}

const nmcli = (args: string[]) => run("nmcli", args)

export async function wifiEnabled(): Promise<boolean> {
  return (await nmcli(["radio", "wifi"])).trim() === "enabled"
}

export function setWifiEnabled(on: boolean): Promise<string> {
  return nmcli(["radio", "wifi", on ? "on" : "off"])
}

export async function listWifi(): Promise<WifiNetwork[]> {
  const saved = parseSavedWifi(await nmcli(["-t", "-f", "NAME,TYPE", "connection", "show"]))
  const list = await nmcli(["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY", "device", "wifi", "list"])
  return parseWifiList(list, saved)
}

export async function listDevices(): Promise<NetDevice[]> {
  const status = parseDeviceStatus(
    await nmcli(["-t", "-f", "DEVICE,TYPE,STATE,CONNECTION", "device", "status"]),
  )
  return Promise.all(
    status.map(async (d) => ({
      ...d,
      addresses:
        d.state === "connected"
          ? parseAddresses(await nmcli(["-t", "-f", "IP4.ADDRESS", "device", "show", d.name]))
          : [],
    })),
  )
}

/**
 * The password goes in nmcli's argument list, so other local users can read it
 * from the process list while nmcli runs; the runtime offers no way to pipe it.
 */
export function connectWifi(ssid: string, password?: string): Promise<string> {
  const args = ["device", "wifi", "connect", ssid]
  if (password) args.push("password", password)
  return nmcli(args)
}

export function disconnectDevice(name: string): Promise<string> {
  return nmcli(["device", "disconnect", name])
}

export function forgetConnection(name: string): Promise<string> {
  return nmcli(["connection", "delete", "id", name])
}
