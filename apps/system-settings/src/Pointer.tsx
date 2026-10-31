import { useCallback, useEffect, useState } from "react"
import { message } from "./shell.ts"
import { readInput, setInput } from "./pointer.ts"
import type { Input } from "./pointer.ts"
import { Card, ErrorBanner, Group, Note, SliderRow, SwitchRow } from "./ui.tsx"

const speedLabel = (v: number) => (v > 0 ? `+${v.toFixed(1)}` : v.toFixed(1))

export default function Pointer() {
  const [input, setValue] = useState<Input | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    readInput().then(setValue, (e) => setError(message(e)))
  }, [])

  /** Change one setting, then read them all back so the page shows what was saved. */
  const change = useCallback(async (key: string, value: boolean | number) => {
    setError(null)
    try {
      await setInput(key, value)
      setValue(await readInput())
    } catch (e) {
      setError(message(e))
    }
  }, [])

  if (!input) return error ? <ErrorBanner>{error}</ErrorBanner> : null
  const { touchpad: t, mouse: m } = input

  return (
    <div className="max-w-[460px]">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <Note>Changes apply at once and are kept for the next login.</Note>
      <Group title="Touchpad">
        <Card>
          <SwitchRow first label="Tap to click" on={t.tapToClick} onChange={(v) => void change("touchpad.tap", v)} />
          <SwitchRow label="Natural scrolling" on={t.naturalScroll} onChange={(v) => void change("touchpad.natural-scroll", v)} />
          <SwitchRow label="Disable while typing" on={t.disableWhileTyping} onChange={(v) => void change("touchpad.disable-while-typing", v)} />
          <SliderRow label="Pointer speed" value={t.pointerSpeed} min={-1} max={1} step={0.1} format={speedLabel} onChange={(v) => void change("touchpad.speed", v)} />
        </Card>
      </Group>
      <Group title="Mouse">
        <Card>
          <SwitchRow first label="Natural scrolling" on={m.naturalScroll} onChange={(v) => void change("mouse.natural-scroll", v)} />
          <SwitchRow label="Left-handed (swap buttons)" on={m.leftHanded} onChange={(v) => void change("mouse.left-handed", v)} />
          <SwitchRow label="Constant speed (no acceleration)" on={m.flatAcceleration} onChange={(v) => void change("mouse.flat-acceleration", v)} />
          <SliderRow label="Pointer speed" value={m.pointerSpeed} min={-1} max={1} step={0.1} format={speedLabel} onChange={(v) => void change("mouse.speed", v)} />
        </Card>
      </Group>
    </div>
  )
}
