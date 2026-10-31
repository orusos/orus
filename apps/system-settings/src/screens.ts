/** Screens through `wlr-randr`; the choice is kept by `orus-appearance display set`. */

import { run } from "./shell.ts"

export interface Mode {
  width: number
  height: number
  /** Hertz; 0 when the screen does not report one. */
  refresh: number
  current: boolean
}

export type Transform = "normal" | "90" | "180" | "270"

export interface Output {
  name: string
  description: string
  enabled: boolean
  scale: number
  transform: string
  /** Top-left corner in the shared layout, in logical pixels. */
  x: number
  y: number
  modes: Mode[]
}

interface RandrOutput {
  name: string
  description?: string | null
  enabled: boolean
  scale: number
  transform?: string
  position?: { x: number; y: number }
  modes: { width: number; height: number; refresh: number; current: boolean }[]
}

/** Parse `wlr-randr --json`; switched-off outputs stay in the list so they can be turned on. */
export function parseOutputs(stdout: string): Output[] {
  return (JSON.parse(stdout) as RandrOutput[]).map((o) => ({
    name: o.name,
    description: o.description ?? o.name,
    enabled: o.enabled,
    scale: o.scale,
    transform: o.transform ?? "normal",
    x: o.position?.x ?? 0,
    y: o.position?.y ?? 0,
    modes: o.modes.map((m) => ({
      width: m.width,
      height: m.height,
      refresh: m.refresh,
      current: m.current,
    })),
  }))
}

/** What an output takes up in the layout: its mode, turned if rotated, divided by the scale. */
export function logicalSize(o: Output): { width: number; height: number } {
  const mode = o.modes.find((m) => m.current) ?? o.modes[0]
  if (!mode) return { width: 0, height: 0 }
  const turned = o.transform.includes("90") || o.transform.includes("270")
  const [w, h] = turned ? [mode.height, mode.width] : [mode.width, mode.height]
  return { width: Math.round(w / o.scale), height: Math.round(h / o.scale) }
}

export type Side = "right" | "left" | "above" | "below"

/** Where `self` goes to sit on `side` of `other`, edge to edge, tops or left sides level. */
export function placement(self: Output, other: Output, side: Side): { x: number; y: number } {
  const mine = logicalSize(self)
  const theirs = logicalSize(other)
  switch (side) {
    case "right":
      return { x: other.x + theirs.width, y: other.y }
    case "left":
      return { x: other.x - mine.width, y: other.y }
    case "above":
      return { x: other.x, y: other.y - mine.height }
    case "below":
      return { x: other.x, y: other.y + theirs.height }
  }
}

/** The `--mode` text `wlr-randr` takes, which `orus-appearance display set` takes too. */
export function modeArg(m: Mode): string {
  return m.refresh > 0 ? `${m.width}x${m.height}@${m.refresh.toFixed(3)}` : `${m.width}x${m.height}`
}

export function modeLabel(m: Mode): string {
  const hertz = m.refresh > 0 ? ` · ${Math.round(m.refresh)} Hz` : ""
  return `${m.width} × ${m.height}${hertz}`
}

/** The highest refresh rate of each size, largest size first. */
export function sizes(modes: Mode[]): Mode[] {
  const best = new Map<string, Mode>()
  for (const m of modes) {
    const key = `${m.width}x${m.height}`
    const seen = best.get(key)
    if (!seen || m.current || (!seen.current && m.refresh > seen.refresh)) best.set(key, m)
  }
  return [...best.values()].sort((a, b) => b.width * b.height - a.width * a.height)
}

export async function readOutputs(): Promise<Output[]> {
  return parseOutputs(await run("wlr-randr", ["--json"]))
}

/** What can change on one output; what is left out stays as it is. */
export interface Change {
  mode?: Mode
  /** A size the screen does not list, as `parseCustomMode` gives it. */
  customMode?: string
  scale?: number
  transform?: Transform
  enabled?: boolean
  position?: { x: number; y: number }
}

/** The `orus-appearance display set` arguments for `change`. */
export function changeArgs(output: string, c: Change): string[] {
  const args = ["display", "set", output]
  if (c.mode) args.push("--mode", modeArg(c.mode))
  if (c.customMode) args.push("--custom-mode", c.customMode)
  if (c.scale !== undefined) args.push("--scale", String(c.scale))
  if (c.transform) args.push("--transform", c.transform)
  if (c.position) args.push("--pos", `${c.position.x},${c.position.y}`)
  if (c.enabled !== undefined) args.push(c.enabled ? "--on" : "--off")
  return args
}

export function apply(output: string, change: Change): Promise<string> {
  return run("orus-appearance", changeArgs(output, change))
}

export const SCALES = [1, 1.25, 1.5, 2]

/** `2560x1600`, `2560 × 1600` or `2560x1600@60` as `2560x1600[@60]`; `null` if it is neither. */
export function parseCustomMode(text: string): string | null {
  const m = /^\s*(\d{3,5})\s*[x×]\s*(\d{3,5})\s*(?:@\s*(\d{2,3}(?:\.\d+)?))?\s*$/i.exec(text)
  if (!m) return null
  const [width, height] = [Number(m[1]), Number(m[2])]
  if (width < 640 || height < 480 || width > 16384 || height > 16384) return null
  return m[3] ? `${width}x${height}@${m[3]}` : `${width}x${height}`
}

/** The change that puts `o` back as it is now, for a revert. */
export function restoreChange(o: Output): Change {
  const mode = o.modes.find((m) => m.current)
  return {
    ...(mode ? { mode } : {}),
    scale: o.scale,
    transform: o.transform as Transform,
    position: { x: o.x, y: o.y },
    enabled: o.enabled,
  }
}

/** Seconds the user has to confirm a display change before it is undone. */
export const CONFIRM_SECONDS = 15
