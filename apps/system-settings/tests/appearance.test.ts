import { describe, expect, it } from "vitest"
import { DEFAULT_APPEARANCE, changes, parseAppearance } from "../src/appearance.ts"

const stdout = JSON.stringify({
  theme: "light",
  accent: "rose",
  reduceMotion: true,
  wallpaper: "dunes",
  effectiveWallpaper: "dunes",
  colors: { accent: "#e06e8f", selection: "#f0dbe2" },
  swatches: { teal: "#1f8285", blue: "#538dd9", purple: "#8c65cd", orange: "#e29c6d", rose: "#e06e8f" },
})

describe("parseAppearance", () => {
  it("reads orus-appearance get output", () => {
    const a = parseAppearance(stdout)
    expect(a).toMatchObject({ theme: "light", accent: "rose", reduceMotion: true, wallpaper: "dunes" })
    expect(a.swatches.rose).toBe("#e06e8f")
  })

  it("treats a null wallpaper as following the theme", () => {
    const a = parseAppearance(stdout.replace('"wallpaper":"dunes"', '"wallpaper":null'))
    expect(a.wallpaper).toBeNull()
  })

  it("rejects values it does not know instead of guessing", () => {
    expect(() => parseAppearance(stdout.replace('"light"', '"sepia"'))).toThrow()
    expect(() => parseAppearance(stdout.replace('"swatches"', '"other"'))).toThrow()
    expect(() => parseAppearance("not json")).toThrow()
  })
})

describe("changes", () => {
  it("lists only what differs, as set arguments", () => {
    const next = { ...DEFAULT_APPEARANCE, theme: "light" as const, reduceMotion: true, wallpaper: "dunes-light" as const }
    expect(changes(DEFAULT_APPEARANCE, next)).toEqual([
      ["theme", "light"],
      ["reduce-motion", "on"],
      ["wallpaper", "dunes-light"],
    ])
    expect(changes(next, { ...next, wallpaper: null })).toEqual([["wallpaper", "auto"]])
    expect(changes(next, next)).toEqual([])
  })
})
