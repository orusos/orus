import { describe, expect, it } from "vitest"
import { displayName, isUnnamed, parseController, parseDeviceList, parseInfo } from "../src/bluetooth.ts"
import { parseInput } from "../src/pointer.ts"
import { changeArgs, logicalSize, parseCustomMode, restoreChange, modeArg, modeLabel, parseOutputs, placement, sizes } from "../src/screens.ts"
import { parseName, parseVolume } from "../src/sound.ts"

describe("bluetooth", () => {
  it("reads the controller, and says none when there is no adapter", () => {
    const show = "Controller AA:BB:CC:DD:EE:FF (public)\n\tName: orus\n\tPowered: yes\n\tDiscovering: no\n"
    expect(parseController(show)).toEqual({ powered: true, discovering: false })
    expect(parseController("No default controller available\n")).toBeNull()
  })

  it("lists devices, keeping names with spaces and dropping other lines", () => {
    const out = "Device 00:11:22:33:44:55 MX Master 3\n[NEW] Device x\nDevice bad-address Foo\nDevice 11-22 Foo\n"
    expect(parseDeviceList(out)).toEqual([{ address: "00:11:22:33:44:55", name: "MX Master 3" }])
  })

  it("reads paired, connected and the icon from info", () => {
    const info = "Device 00:11:22:33:44:55 (public)\n\tName: x\n\tIcon: input-mouse\n\tPaired: yes\n\tTrusted: yes\n\tConnected: no\n"
    expect(parseInfo(info)).toEqual({ paired: true, connected: false, icon: "input-mouse" })
  })

  it("calls a device named after its address unnamed", () => {
    expect(displayName({ address: "00:11:22:33:44:55", name: "00-11-22-33-44-55" })).toBe("Unnamed device")
    expect(displayName({ address: "00:11:22:33:44:55", name: "Buds" })).toBe("Buds")
    expect(isUnnamed({ address: "00:11:22:33:44:55", name: "00-11-22-33-44-55" })).toBe(true)
  })
})

describe("sound", () => {
  it("reads volume and mute", () => {
    expect(parseVolume("Volume: 0.40\n")).toEqual({ percent: 40, muted: false })
    expect(parseVolume("Volume: 0.75 [MUTED]\n")).toEqual({ percent: 75, muted: true })
    expect(parseVolume("Volume: 1.20\n")?.percent).toBe(100)
    expect(parseVolume("")).toBeNull()
  })

  it("reads the device name", () => {
    expect(parseName('  * node.description = "Built-in Audio Analog Stereo"\n')).toBe("Built-in Audio Analog Stereo")
    expect(parseName("")).toBe("")
  })
})

describe("screens", () => {
  const json = JSON.stringify([
    {
      name: "Virtual-1",
      description: "Virtual screen",
      enabled: true,
      scale: 1,
      modes: [
        { width: 1920, height: 1080, refresh: 60, preferred: true, current: true },
        { width: 1920, height: 1080, refresh: 30, preferred: false, current: false },
        { width: 1280, height: 800, refresh: 59.934, preferred: false, current: false },
      ],
    },
    { name: "Off-1", enabled: false, scale: 1, modes: [] },
  ])

  it("keeps switched-off outputs so they can be turned on", () => {
    expect(parseOutputs(json).map((o) => [o.name, o.enabled])).toEqual([
      ["Virtual-1", true],
      ["Off-1", false],
    ])
  })

  it("keeps the best refresh of each size, largest first, and the current one over a faster one", () => {
    const [output] = parseOutputs(json)
    expect(sizes(output.modes).map(modeLabel)).toEqual(["1920 × 1080 · 60 Hz", "1280 × 800 · 60 Hz"])
  })

  it("writes the mode the way wlr-randr takes it", () => {
    expect(modeArg({ width: 1280, height: 800, refresh: 59.934, current: false })).toBe("1280x800@59.934")
    expect(modeArg({ width: 1280, height: 720, refresh: 0, current: true })).toBe("1280x720")
  })
})

describe("screen layout", () => {
  const mode = (w: number, h: number) => [{ width: w, height: h, refresh: 60, current: true }]
  const out = (name: string, w: number, h: number, extra: object = {}) => ({
    name,
    description: name,
    enabled: true,
    scale: 1,
    transform: "normal",
    x: 0,
    y: 0,
    modes: mode(w, h),
    ...extra,
  })

  it("sizes an output as the layout sees it: rotated and scaled", () => {
    expect(logicalSize(out("a", 1920, 1080))).toEqual({ width: 1920, height: 1080 })
    expect(logicalSize(out("a", 1920, 1080, { transform: "90" }))).toEqual({ width: 1080, height: 1920 })
    expect(logicalSize(out("a", 2560, 1440, { scale: 2 }))).toEqual({ width: 1280, height: 720 })
  })

  it("places a screen edge to edge on each side of another", () => {
    const main = out("main", 1920, 1080, { x: 100, y: 50 })
    const side = out("side", 1280, 720)
    expect(placement(side, main, "right")).toEqual({ x: 2020, y: 50 })
    expect(placement(side, main, "left")).toEqual({ x: -1180, y: 50 })
    expect(placement(side, main, "above")).toEqual({ x: 100, y: -670 })
    expect(placement(side, main, "below")).toEqual({ x: 100, y: 1130 })
  })

  it("turns a change into orus-appearance arguments", () => {
    expect(
      changeArgs("HDMI-A-1", { transform: "90", position: { x: 1920, y: 0 }, enabled: true }),
    ).toEqual(["display", "set", "HDMI-A-1", "--transform", "90", "--pos", "1920,0", "--on"])
    expect(changeArgs("HDMI-A-1", { enabled: false })).toEqual(["display", "set", "HDMI-A-1", "--off"])
  })
})

describe("undoing a display change", () => {
  it("restores the current mode, scale, rotation, position and state", () => {
    const o = parseOutputs(
      JSON.stringify([
        {
          name: "V-1",
          enabled: true,
          scale: 1.5,
          transform: "90",
          position: { x: 10, y: 20 },
          modes: [
            { width: 1920, height: 1080, refresh: 60, current: false },
            { width: 1280, height: 800, refresh: 75, current: true },
          ],
        },
      ]),
    )[0]
    expect(changeArgs("V-1", restoreChange(o))).toEqual([
      "display", "set", "V-1", "--mode", "1280x800@75.000", "--scale", "1.5",
      "--transform", "90", "--pos", "10,20", "--on",
    ])
  })
})

describe("custom size", () => {
  it("accepts a typed size, with an x or ×, spaces and an optional rate", () => {
    expect(parseCustomMode("2560x1600")).toBe("2560x1600")
    expect(parseCustomMode(" 2560 × 1600 ")).toBe("2560x1600")
    expect(parseCustomMode("2880x1800@60")).toBe("2880x1800@60")
  })

  it("refuses text that is not a size, or is absurd", () => {
    for (const bad of ["", "wide", "2560", "2560x", "100x100", "99999x99999", "2560x1600; reboot"]) {
      expect(parseCustomMode(bad)).toBeNull()
    }
  })

  it("passes it to orus-appearance as a custom mode", () => {
    expect(changeArgs("Virtual-1", { customMode: "2560x1600" })).toEqual([
      "display", "set", "Virtual-1", "--custom-mode", "2560x1600",
    ])
  })
})

describe("pointer", () => {
  const ok = {
    touchpad: { tapToClick: true, naturalScroll: true, pointerSpeed: 0, disableWhileTyping: true },
    mouse: { naturalScroll: false, pointerSpeed: 0.5, leftHanded: false, flatAcceleration: false },
  }
  it("accepts a complete setting and refuses a partial one", () => {
    expect(parseInput(JSON.stringify(ok))).toEqual(ok)
    expect(() => parseInput(JSON.stringify({ touchpad: ok.touchpad }))).toThrow()
  })
})

import { nameFromUri, parseDefault, parseFound, parseStates, parseUris, queueName } from "../src/printers.ts"

describe("printers", () => {
  it("reads state, device and default from lpstat", () => {
    const p = [
      "printer HP_Laser is idle.  enabled since Tue 20 Oct 2026",
      "printer Office now printing Office-3.  enabled since Tue",
      "printer Old disabled since Tue -",
      "\treason",
    ].join("\n")
    expect([...parseStates(p)]).toEqual([
      ["HP_Laser", "idle"],
      ["Office", "printing"],
      ["Old", "disabled"],
    ])
    expect(parseUris("device for HP_Laser: dnssd://HP%20Laser._ipp._tcp.local/\n").get("HP_Laser")).toBe(
      "dnssd://HP%20Laser._ipp._tcp.local/",
    )
    expect(parseDefault("system default destination: HP_Laser\n")).toBe("HP_Laser")
    expect(parseDefault("no system default destination\n")).toBeNull()
  })

  it("keeps only dnssd and usb finds, minus printers already set up", () => {
    const out = [
      "network beh",
      "network ipp",
      "network socket",
      "network dnssd://Brother%20HL-L2350DW._ipp._tcp.local/?uuid=abc",
      "network dnssd://Known._ipp._tcp.local/",
      "direct usb://HP/LaserJet%20M15?serial=1",
      "file cups-pdf:/",
    ].join("\n")
    const found = parseFound(out, new Set(["dnssd://Known._ipp._tcp.local/"]))
    expect(found).toEqual([
      { uri: "dnssd://Brother%20HL-L2350DW._ipp._tcp.local/?uuid=abc", name: "Brother HL-L2350DW", kind: "network" },
      { uri: "usb://HP/LaserJet%20M15?serial=1", name: "HP LaserJet M15", kind: "usb" },
    ])
  })

  it("names queues with characters CUPS accepts", () => {
    expect(queueName("Brother HL-L2350DW")).toBe("Brother_HL-L2350DW")
    expect(queueName("  ä ö  ")).toBe("Printer")
    expect(nameFromUri("dnssd://Bad%ZZname._ipp._tcp.local/")).toBe("Bad%ZZname")
  })
})
