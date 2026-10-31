/** The `orus` command, run through the runtime's `host.shell` capability. */

import { exec, run } from "./shell.ts"

/** Downloading and unpacking an app; the runtime stops a command after 60 s. */
const INSTALL_TIMEOUT_MS = 60_000

/** An app a source offers. */
export interface Offer {
  id: string
  name: string
  version: string
  description: string
  capabilities: string[]
  size: number
  source: string
  /** How far the source is trusted, in words. */
  trust: string
  unsigned: boolean
  /** The version installed, if any. */
  installed: string | null
}

export interface Installed {
  id: string
  name: string
  version: string
  description: string
  capabilities: string[]
  /** An earlier version is kept to go back to. */
  canRollBack: boolean
  /// Part of Orus OS: the Store lists it nowhere and cannot remove it.
  system: boolean
}

export interface Source {
  name: string
  url: string
  development: boolean
  system: boolean
  enabled: boolean
  signed: boolean
}

const CAPABILITY_LABELS: Record<string, string> = {
  "media.camera.capture": "Use the camera",
  "media.microphone.capture": "Use the microphone",
  "media.audio.playback": "Play audio",
  "media.video.playback": "Play video",
  "host.shell": "Run commands on this computer",
  "host.fs": "Read and write files",
}

export function capabilityLabel(id: string): string {
  return CAPABILITY_LABELS[id] ?? id
}

/** Whether version `a` is newer than `b`, comparing dot-separated numbers. */
export function isNewer(a: string, b: string): boolean {
  const left = a.split(".").map((part) => parseInt(part, 10) || 0)
  const right = b.split(".").map((part) => parseInt(part, 10) || 0)
  for (let i = 0; i < Math.max(left.length, right.length); i++) {
    const diff = (left[i] ?? 0) - (right[i] ?? 0)
    if (diff !== 0) return diff > 0
  }
  return false
}

export function formatSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

/** `name@version`: the version pins the install to what the user was shown. */
export function spec(offer: Pick<Offer, "id" | "version">): string {
  return `${offer.id}@${offer.version}`
}

const orus = (args: string[], timeoutMs?: number) => run("orus", args, timeoutMs)

export async function search(text: string): Promise<Offer[]> {
  return JSON.parse(await orus(["search", "--json", ...(text ? [text] : [])])) as Offer[]
}

export async function installed(): Promise<Installed[]> {
  return JSON.parse(await orus(["list", "--json"])) as Installed[]
}

export async function sources(): Promise<Source[]> {
  return JSON.parse(await orus(["sources", "list", "--json"])) as Source[]
}

/** Read every enabled source's catalog again. Returns what failed, one line per source. */
export async function refresh(): Promise<string[]> {
  const result = await exec("orus", ["update"], INSTALL_TIMEOUT_MS)
  if (!result.success) throw new Error(result.stderr.trim() || "orus update failed")
  return result.stderr.split("\n").filter((line) => line.trim().length > 0)
}

/** Install `offer`; the command refuses if the app asks for more than `offer` shows. */
export async function install(offer: Offer): Promise<void> {
  await orus(["install", "--allow", offer.capabilities.join(","), spec(offer)], INSTALL_TIMEOUT_MS)
}

export const open = (id: string) => orus(["open", id])
export const remove = (id: string) => orus(["remove", id])
export const purge = (id: string) => orus(["purge", id], INSTALL_TIMEOUT_MS)
export const rollback = (id: string) => orus(["rollback", id])

export function addSource(url: string, development: boolean, key: string | null) {
  return orus([
    "sources",
    "add",
    ...(development ? ["--development"] : []),
    ...(key ? ["--key", key] : []),
    url,
  ])
}

export const removeSource = (name: string) => orus(["sources", "remove", name])
export const setSourceEnabled = (name: string, on: boolean) =>
  orus(["sources", on ? "enable" : "disable", name])
