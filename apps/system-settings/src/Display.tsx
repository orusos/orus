import { useCallback, useEffect, useState } from "react"
import { message } from "./shell.ts"
import * as screens from "./screens.ts"
import type { Change, Output, Side, Transform } from "./screens.ts"
import { Button, Card, ErrorBanner, Group, Note, Row, Select, Switch } from "./ui.tsx"

const ROTATIONS: { value: Transform; label: string }[] = [
  { value: "normal", label: "Normal" },
  { value: "90", label: "90° right" },
  { value: "180", label: "Upside down" },
  { value: "270", label: "90° left" },
]

const SIDES: { value: Side; label: string }[] = [
  { value: "right", label: "Right of" },
  { value: "left", label: "Left of" },
  { value: "above", label: "Above" },
  { value: "below", label: "Below" },
]

function Screen({
  output,
  others,
  onlyOn,
  busy,
  change,
}: {
  output: Output
  others: Output[]
  /** This is the only screen that is on, so it cannot be turned off. */
  onlyOn: boolean
  busy: boolean
  change: (name: string, c: Change) => void
}) {
  const sizes = screens.sizes(output.modes)
  const current = output.modes.find((m) => m.current) ?? sizes[0]
  const [anchor, setAnchor] = useState(others[0]?.name ?? "")
  const [custom, setCustom] = useState("")
  const customMode = screens.parseCustomMode(custom)
  const target = others.find((o) => o.name === anchor) ?? others[0]
  const disabled = busy || !output.enabled

  return (
    <Group title={output.description}>
      <Card>
        <Row first>
          <span className="flex-1 text-[14px]">On</span>
          <Switch
            on={output.enabled}
            label={`${output.description} on`}
            disabled={busy || (onlyOn && output.enabled)}
            onChange={(on) => change(output.name, { enabled: on })}
          />
        </Row>
        <Row>
          <span className="flex-1 text-[14px]">Resolution</span>
          <Select
            label="Resolution"
            disabled={disabled || sizes.length === 0}
            value={current ? screens.modeArg(current) : ""}
            choices={sizes.map((m) => ({ value: screens.modeArg(m), label: screens.modeLabel(m) }))}
            onChange={(arg) => {
              const mode = output.modes.find((m) => screens.modeArg(m) === arg)
              if (mode) change(output.name, { mode })
            }}
          />
        </Row>
        <Row>
          <span className="flex-1 text-[14px]">Custom size</span>
          <input
            aria-label="Custom size"
            value={custom}
            disabled={disabled}
            placeholder="2560x1600"
            onChange={(e) => setCustom(e.target.value)}
            className="w-[110px] rounded-lg bg-fill px-3 py-1 text-[13px] text-text outline-none placeholder:text-dim"
          />
          <Button
            disabled={disabled || customMode === null}
            onClick={() => customMode && change(output.name, { customMode })}
          >
            Apply
          </Button>
        </Row>
        <Row>
          <span className="flex-1 text-[14px]">Scale</span>
          <div className="flex gap-1.5">
            {screens.SCALES.map((s) => (
              <Button
                key={s}
                primary={Math.abs(output.scale - s) < 0.01}
                disabled={disabled}
                onClick={() => change(output.name, { scale: s })}
              >
                {`${Math.round(s * 100)}%`}
              </Button>
            ))}
          </div>
        </Row>
        <Row>
          <span className="flex-1 text-[14px]">Rotation</span>
          <Select
            label="Rotation"
            disabled={disabled}
            value={output.transform}
            choices={ROTATIONS}
            onChange={(value) => change(output.name, { transform: value as Transform })}
          />
        </Row>
        {target && (
          <Row>
            <span className="flex-1 text-[14px]">Position</span>
            <Select
              label="Side"
              disabled={disabled}
              value=""
              placeholder="Place…"
              choices={SIDES}
              onChange={(side) =>
                change(output.name, { position: screens.placement(output, target, side as Side) })
              }
            />
            <Select
              label="Next to"
              disabled={disabled || others.length < 2}
              value={target.name}
              choices={others.map((o) => ({ value: o.name, label: o.description }))}
              onChange={setAnchor}
            />
          </Row>
        )}
      </Card>
    </Group>
  )
}

export default function Display() {
  const [outputs, setOutputs] = useState<Output[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)
  /** A change waiting to be confirmed, and how to undo it. */
  const [pending, setPending] = useState<{ output: string; undo: Change; left: number } | null>(
    null,
  )

  const load = useCallback(async () => {
    try {
      setOutputs(await screens.readOutputs())
    } catch (e) {
      setError(message(e))
    }
  }, [])

  useEffect(() => {
    void load()
  }, [load])

  async function change(name: string, c: Change) {
    const before = outputs?.find((o) => o.name === name)
    setBusy(true)
    setError(null)
    try {
      await screens.apply(name, c)
      // A size the screen cannot show well would leave nothing to click, so the change
      // is undone unless it is kept.
      if (before) setPending({ output: name, undo: screens.restoreChange(before), left: screens.CONFIRM_SECONDS })
    } catch (e) {
      setError(message(e))
    }
    await load()
    setBusy(false)
  }

  async function revert() {
    if (!pending) return
    const { output, undo } = pending
    setPending(null)
    setBusy(true)
    try {
      await screens.apply(output, undo)
    } catch (e) {
      setError(message(e))
    }
    await load()
    setBusy(false)
  }

  const counting = pending !== null
  useEffect(() => {
    if (!counting) return
    const timer = setInterval(
      () => setPending((p) => (p ? { ...p, left: p.left - 1 } : p)),
      1000,
    )
    return () => clearInterval(timer)
  }, [counting])

  useEffect(() => {
    if (pending && pending.left <= 0) void revert()
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pending?.left])

  const on = outputs?.filter((o) => o.enabled) ?? []

  return (
    <div className="max-w-[460px]">
      {pending && (
        <div className="sticky top-0 z-30 mb-3 flex items-center gap-3 rounded-xl border border-card-line bg-card px-4 py-3">
          <span className="flex-1 text-[14px]">Keep these display settings? Going back in {pending.left}s.</span>
          <Button primary onClick={() => setPending(null)}>
            Keep
          </Button>
          <Button onClick={() => void revert()}>Go back</Button>
        </div>
      )}
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <Note>Changes apply at once; keep them within 15 seconds or they are undone. A size the list lacks can be typed under Custom size; a real screen may refuse it.</Note>
      {outputs?.length === 0 && <Note>No screens found.</Note>}
      {outputs?.map((o) => (
        <Screen
          key={o.name}
          output={o}
          others={on.filter((x) => x.name !== o.name)}
          onlyOn={on.length <= 1}
          busy={busy}
          change={(n, c) => void change(n, c)}
        />
      ))}
    </div>
  )
}
