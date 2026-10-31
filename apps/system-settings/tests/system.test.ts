import { describe, expect, it } from "vitest"
import { parseAbout, parsePower } from "../src/system.ts"

describe("parsePower", () => {
  it("reads each battery line and caps the percent at 100", () => {
    expect(parsePower("BAT0|73|Charging\nBAT1|104|Full\n")).toEqual([
      { name: "BAT0", percent: 73, status: "Charging" },
      { name: "BAT1", percent: 100, status: "Full" },
    ])
  })

  it("returns nothing for no batteries or malformed lines", () => {
    expect(parsePower("")).toEqual([])
    expect(parsePower("BAT0|n/a|Unknown\nnonsense\n")).toEqual([])
  })
})

describe("parseAbout", () => {
  const out = [
    "host=orus",
    "kernel=6.12.43+deb13-arm64",
    "arch=aarch64",
    "os=Debian GNU/Linux 13 (trixie)",
    "mem=4048560",
    "version=0.1.0",
  ].join("\n")

  it("reads the key=value lines and converts memory to GiB", () => {
    expect(parseAbout(out)).toEqual({
      hostname: "orus",
      os: "Debian GNU/Linux 13 (trixie)",
      kernel: "6.12.43+deb13-arm64",
      arch: "aarch64",
      memoryGiB: "3.9",
      version: "0.1.0",
    })
  })

  it("leaves a field empty when its line is missing, and keeps '=' inside a value", () => {
    const info = parseAbout("host=a=b\nmem=\n")
    expect(info.hostname).toBe("a=b")
    expect(info.memoryGiB).toBe("")
    expect(info.version).toBe("")
  })
})
