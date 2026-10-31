import { useCallback, useEffect, useState } from "react"
import { Mic, Volume2 } from "lucide-react"
import { message } from "./shell.ts"
import * as sound from "./sound.ts"
import type { Level, Side } from "./sound.ts"
import { Card, ErrorBanner, Group, Note, SliderRow, SwitchRow } from "./ui.tsx"

const percent = (v: number) => `${Math.round(v)}%`

function Device({
  side,
  title,
  level,
  change,
}: {
  side: Side
  title: string
  level: Level | null
  change: (run: () => Promise<unknown>) => void
}) {
  const Icon = side === "output" ? Volume2 : Mic
  return (
    <Group title={title}>
      {level === null ? (
        <Card>
          <li className="px-4 py-3 text-[13px] text-dim">
            No {side === "output" ? "speaker" : "microphone"} found.
          </li>
        </Card>
      ) : (
        <Card>
          <li className="flex items-center gap-3 px-4 py-3 text-[13px] text-dim">
            <Icon size={18} strokeWidth={1.6} />
            {level.name || "Default device"}
          </li>
          <SliderRow label="Volume" value={level.percent} min={0} max={100} step={1} format={percent} onChange={(v) => change(() => sound.setVolume(side, v))} />
          <SwitchRow label="Mute" on={level.muted} onChange={(on) => change(() => sound.setMuted(side, on))} />
        </Card>
      )}
    </Group>
  )
}

export default function Sound() {
  const [output, setOutput] = useState<Level | null>(null)
  const [input, setInput] = useState<Level | null>(null)
  const [error, setError] = useState<string | null>(null)

  const load = useCallback(async () => {
    try {
      setOutput(await sound.read("output"))
      setInput(await sound.read("input"))
    } catch (e) {
      setError(message(e))
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  const change = useCallback(
    (run: () => Promise<unknown>) => {
      setError(null)
      run().then(load, (e) => setError(message(e)))
    },
    [load],
  )

  return (
    <div className="max-w-[460px]">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <Note>Applies to the default devices.</Note>
      <Device side="output" title="Output" level={output} change={change} />
      <Device side="input" title="Input" level={input} change={change} />
    </div>
  )
}
