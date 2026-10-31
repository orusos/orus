/** Printers through CUPS (`lpstat`, `lpinfo`, `lpadmin`, `lp`), run with `host.shell`. */

import { run } from "./shell.ts"

export interface Printer {
  name: string
  /** `idle`, `printing` or `disabled`. */
  state: string
  isDefault: boolean
  uri: string
}

/** A printer CUPS found that is not set up yet. */
export interface Found {
  uri: string
  name: string
  /** `network` or `usb`. */
  kind: "network" | "usb"
}

// CUPS keeps its admin tools in /usr/sbin, which is not on a user's PATH.
const LPINFO = "/usr/sbin/lpinfo"
const LPADMIN = "/usr/sbin/lpadmin"
const TEST_PAGE = "/usr/share/cups/data/testprint"
const DISCOVERY_MS = 40_000

/** `lpstat -p`: `printer NAME is idle.` / `printer NAME now printing NAME-3.` / `... disabled ...`. */
export function parseStates(stdout: string): Map<string, string> {
  const states = new Map<string, string>()
  for (const line of stdout.split("\n")) {
    const m = /^printer (\S+) (.*)$/.exec(line)
    if (!m) continue
    const rest = m[2]
    states.set(m[1], rest.startsWith("is idle") ? "idle" : rest.startsWith("now printing") ? "printing" : "disabled")
  }
  return states
}

/** `lpstat -v`: `device for NAME: URI`. */
export function parseUris(stdout: string): Map<string, string> {
  const uris = new Map<string, string>()
  for (const line of stdout.split("\n")) {
    const m = /^device for (\S+): (.*)$/.exec(line)
    if (m) uris.set(m[1], m[2].trim())
  }
  return uris
}

/** `lpstat -d`: `system default destination: NAME`; `null` when there is none. */
export function parseDefault(stdout: string): string | null {
  return /^system default destination: (\S+)/m.exec(stdout)?.[1] ?? null
}

/** A readable name for the device URI: the service name of a `dnssd://` address. */
export function nameFromUri(uri: string): string {
  const dnssd = /^dnssd:\/\/([^/]+?)\._(?:ipp|ipps|printer)/.exec(uri)
  if (dnssd) return safeDecode(dnssd[1])
  const usb = /^usb:\/\/([^?]+)/.exec(uri)
  if (usb) return safeDecode(usb[1]).replaceAll("/", " ")
  return uri
}

function safeDecode(text: string): string {
  try {
    return decodeURIComponent(text)
  } catch {
    return text
  }
}

/**
 * `lpinfo -v`: one `<class> <uri>` line per way to reach a printer. Only the addresses
 * that name one printer are kept (`dnssd://` for the network, `usb://` for a cable); the
 * rest are generic ports such as `socket` and `lpd`. Printers already added are left out.
 */
export function parseFound(stdout: string, known: Set<string>): Found[] {
  const found: Found[] = []
  for (const line of stdout.split("\n")) {
    const m = /^(network|direct) (dnssd|usb):\/\/(\S+)$/.exec(line.trim())
    if (!m) continue
    const uri = `${m[2]}://${m[3]}`
    if (known.has(uri)) continue
    found.push({ uri, name: nameFromUri(uri), kind: m[2] === "usb" ? "usb" : "network" })
  }
  return found
}

/** A queue name CUPS accepts: letters, digits, `-` and `_`. */
export function queueName(display: string): string {
  const name = display.replace(/[^A-Za-z0-9_-]+/g, "_").replace(/^_+|_+$/g, "")
  return name === "" ? "Printer" : name.slice(0, 40)
}

export async function listPrinters(): Promise<Printer[]> {
  const states = parseStates(await run("lpstat", ["-p"]).catch(() => ""))
  const uris = parseUris(await run("lpstat", ["-v"]).catch(() => ""))
  const def = parseDefault(await run("lpstat", ["-d"]).catch(() => ""))
  return [...states].map(([name, state]) => ({
    name,
    state,
    isDefault: name === def,
    uri: uris.get(name) ?? "",
  }))
}

/** Ask CUPS which printers it can reach; takes several seconds. */
export async function detect(known: Printer[]): Promise<Found[]> {
  const uris = new Set(known.map((p) => p.uri))
  return parseFound(await run(LPINFO, ["-v"], DISCOVERY_MS), uris)
}

/** Set up `found` as a driverless (IPP Everywhere) printer and return its queue name. */
export async function add(found: Found, taken: Printer[]): Promise<string> {
  const base = queueName(found.name)
  const used = new Set(taken.map((p) => p.name))
  let name = base
  for (let n = 2; used.has(name); n++) name = `${base}_${n}`
  await run(LPADMIN, ["-p", name, "-E", "-v", found.uri, "-m", "everywhere"], 30_000)
  if (taken.length === 0) await run(LPADMIN, ["-d", name])
  return name
}

export const remove = (name: string) => run(LPADMIN, ["-x", name])
export const makeDefault = (name: string) => run(LPADMIN, ["-d", name])
export const testPage = (name: string) => run("lp", ["-d", name, TEST_PAGE])
