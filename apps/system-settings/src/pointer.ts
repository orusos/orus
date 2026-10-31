/** Mouse and touchpad settings, read and changed through `orus-appearance input`. */

import { run } from "./shell.ts"

export interface Input {
  touchpad: {
    tapToClick: boolean
    naturalScroll: boolean
    pointerSpeed: number
    disableWhileTyping: boolean
  }
  mouse: {
    naturalScroll: boolean
    pointerSpeed: number
    leftHanded: boolean
    flatAcceleration: boolean
  }
}

/** Parse `orus-appearance input get`; throws if it is not a complete setting. */
export function parseInput(stdout: string): Input {
  const v = JSON.parse(stdout) as Partial<Input>
  const t = v.touchpad
  const m = v.mouse
  if (
    typeof t?.tapToClick !== "boolean" ||
    typeof t.naturalScroll !== "boolean" ||
    typeof t.pointerSpeed !== "number" ||
    typeof t.disableWhileTyping !== "boolean" ||
    typeof m?.naturalScroll !== "boolean" ||
    typeof m.pointerSpeed !== "number" ||
    typeof m.leftHanded !== "boolean" ||
    typeof m.flatAcceleration !== "boolean"
  ) {
    throw new Error("orus-appearance printed an unexpected input setting")
  }
  return v as Input
}

export async function readInput(): Promise<Input> {
  return parseInput(await run("orus-appearance", ["input", "get"]))
}

/** `key` is one of the names `orus-appearance input set` takes, such as `touchpad.tap`. */
export function setInput(key: string, value: boolean | number): Promise<string> {
  const text = typeof value === "boolean" ? (value ? "on" : "off") : value.toFixed(2)
  return run("orus-appearance", ["input", "set", key, text])
}
