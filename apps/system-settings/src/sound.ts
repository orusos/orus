/** Volume of the default speaker and microphone through `wpctl` (PipeWire). */

import { exec, run } from "./shell.ts"

export interface Level {
  /** 0-100; the output can go to 100 only. */
  percent: number
  muted: boolean
  name: string
}

export type Side = "output" | "input"

const TARGET: Record<Side, string> = {
  output: "@DEFAULT_AUDIO_SINK@",
  input: "@DEFAULT_AUDIO_SOURCE@",
}

/** `wpctl get-volume`: `Volume: 0.40` or `Volume: 0.40 [MUTED]`. */
export function parseVolume(stdout: string): { percent: number; muted: boolean } | null {
  const m = /Volume: ([0-9.]+)( \[MUTED\])?/.exec(stdout)
  if (!m) return null
  return { percent: Math.round(Math.min(1, Number(m[1])) * 100), muted: m[2] !== undefined }
}

/** The device's description from `wpctl inspect`. */
export function parseName(stdout: string): string {
  return /node\.description = "([^"]*)"/.exec(stdout)?.[1] ?? ""
}

/** The current level, or `null` when there is no such device (no microphone, say). */
export async function read(side: Side): Promise<Level | null> {
  const volume = await exec("wpctl", ["get-volume", TARGET[side]])
  const parsed = volume.success ? parseVolume(volume.stdout) : null
  if (!parsed) return null
  const inspect = await exec("wpctl", ["inspect", TARGET[side]])
  return { ...parsed, name: parseName(inspect.stdout) }
}

export function setVolume(side: Side, percent: number): Promise<string> {
  const clamped = Math.max(0, Math.min(100, Math.round(percent)))
  return run("wpctl", ["set-volume", TARGET[side], (clamped / 100).toFixed(2)])
}

export function setMuted(side: Side, muted: boolean): Promise<string> {
  return run("wpctl", ["set-mute", TARGET[side], muted ? "1" : "0"])
}
