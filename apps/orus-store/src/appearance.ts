/** The shared appearance setting, read through `orus-appearance`. */

import { run } from "./shell.ts"

export type Theme = "dark" | "light"
export type Accent = "teal" | "blue" | "purple" | "orange" | "rose"
export type Wallpaper = "dunes" | "dunes-light"

export interface Appearance {
  theme: Theme
  accent: Accent
  reduceMotion: boolean
  /** `null` follows the theme. */
  wallpaper: Wallpaper | null
  /** The wallpaper in use: the chosen one, else the theme's own. */
  effectiveWallpaper: Wallpaper
  colors: { accent: string; selection: string }
  /** Every accent's color for the current theme. */
  swatches: Record<Accent, string>
}

export const DEFAULT_APPEARANCE: Appearance = {
  theme: "dark",
  accent: "teal",
  reduceMotion: false,
  wallpaper: null,
  effectiveWallpaper: "dunes",
  colors: { accent: "#63b1ac", selection: "#2e4849" },
  swatches: {
    teal: "#63b1ac",
    blue: "#5182ba",
    purple: "#7f6aaf",
    orange: "#d29872",
    rose: "#c66b7e",
  },
}

export const ACCENTS: Accent[] = ["teal", "blue", "purple", "orange", "rose"]

const THEMES: Theme[] = ["dark", "light"]
export const WALLPAPERS: Wallpaper[] = ["dunes", "dunes-light"]

/** Parse `orus-appearance get` output; throws if it is not a complete setting. */
export function parseAppearance(stdout: string): Appearance {
  const v = JSON.parse(stdout) as Partial<Appearance>
  const wallpaper = v.wallpaper ?? null
  if (
    !THEMES.includes(v.theme as Theme) ||
    !ACCENTS.includes(v.accent as Accent) ||
    typeof v.reduceMotion !== "boolean" ||
    (wallpaper !== null && !WALLPAPERS.includes(wallpaper)) ||
    !WALLPAPERS.includes(v.effectiveWallpaper as Wallpaper) ||
    typeof v.colors?.accent !== "string" ||
    typeof v.colors?.selection !== "string" ||
    !ACCENTS.every((a) => typeof v.swatches?.[a] === "string")
  ) {
    throw new Error("orus-appearance printed an unexpected setting")
  }
  return {
    theme: v.theme as Theme,
    accent: v.accent as Accent,
    reduceMotion: v.reduceMotion,
    wallpaper,
    effectiveWallpaper: v.effectiveWallpaper as Wallpaper,
    colors: v.colors as Appearance["colors"],
    swatches: v.swatches as Appearance["swatches"],
  }
}

export async function readAppearance(): Promise<Appearance> {
  return parseAppearance(await run("orus-appearance", ["get"]))
}
