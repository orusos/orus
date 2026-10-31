import { describe, expect, it } from "vitest"
import {
  parseAddresses,
  parseDeviceStatus,
  parseSavedWifi,
  parseWifiList,
  splitTerse,
} from "../src/nmcli.ts"

describe("splitTerse", () => {
  it("splits on colons and unescapes colons and backslashes", () => {
    expect(splitTerse("*:Cafe\\: Free:72:WPA2")).toEqual(["*", "Cafe: Free", "72", "WPA2"])
    expect(splitTerse("a\\\\b:c")).toEqual(["a\\b", "c"])
  })

  it("keeps empty fields", () => {
    expect(splitTerse(" :Home:80:")).toEqual([" ", "Home", "80", ""])
  })
})

describe("parseWifiList", () => {
  const out = [
    " :Home:60:WPA2",
    "*:Home:81:WPA2",
    " :Guest:40:",
    " ::55:WPA2",
    " :Neighbour:90:WPA2 WPA3",
  ].join("\n")

  it("keeps one entry per SSID, drops hidden ones, and puts the active network first", () => {
    const list = parseWifiList(out, new Set())
    expect(list.map((n) => n.ssid)).toEqual(["Home", "Neighbour", "Guest"])
    expect(list[0]).toMatchObject({ active: true, signal: 81 })
  })

  it("marks open networks and saved ones", () => {
    const list = parseWifiList(out, new Set(["Neighbour"]))
    expect(list.find((n) => n.ssid === "Guest")?.security).toBe("")
    expect(list.find((n) => n.ssid === "Neighbour")?.saved).toBe(true)
    expect(list.find((n) => n.ssid === "Guest")?.saved).toBe(false)
  })
})

describe("device and connection output", () => {
  it("keeps ethernet and wifi devices only", () => {
    const out = ["enp0s1:ethernet:connected:Wired connection 1", "lo:loopback:unmanaged:", "wlan0:wifi:disconnected:"].join("\n")
    expect(parseDeviceStatus(out)).toEqual([
      { name: "enp0s1", type: "ethernet", state: "connected", connection: "Wired connection 1" },
      { name: "wlan0", type: "wifi", state: "disconnected", connection: "" },
    ])
  })

  it("reads IPv4 addresses", () => {
    expect(parseAddresses("IP4.ADDRESS[1]:192.168.64.7/24\nIP4.ADDRESS[2]:10.0.0.2/8")).toEqual([
      "192.168.64.7/24",
      "10.0.0.2/8",
    ])
  })

  it("lists saved wifi connections by name", () => {
    const out = "Home:802-11-wireless\nWired connection 1:802-3-ethernet"
    expect([...parseSavedWifi(out)]).toEqual(["Home"])
  })
})
