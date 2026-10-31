import { describe, expect, it } from "vitest"
import { capabilityLabel, formatSize, isNewer, spec } from "../src/orus.ts"

describe("isNewer", () => {
  it("compares by number, not by text", () => {
    expect(isNewer("1.10.0", "1.9.0")).toBe(true)
    expect(isNewer("1.9.0", "1.10.0")).toBe(false)
  })
  it("treats a missing part as zero and equal versions as not newer", () => {
    expect(isNewer("1.0", "1.0.0")).toBe(false)
    expect(isNewer("1.0.1", "1.0")).toBe(true)
  })
})

describe("labels", () => {
  it("names known capabilities and shows unknown ones as they are", () => {
    expect(capabilityLabel("media.camera.capture")).toBe("Use the camera")
    expect(capabilityLabel("x.y")).toBe("x.y")
  })
  it("sizes", () => {
    expect(formatSize(900)).toBe("900 B")
    expect(formatSize(2048)).toBe("2 KB")
    expect(formatSize(3 * 1024 * 1024)).toBe("3.0 MB")
  })
  it("pins the install to the shown version", () => {
    expect(spec({ id: "hello", version: "1.2.0" })).toBe("hello@1.2.0")
  })
})
