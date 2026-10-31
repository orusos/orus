import { useCallback, useEffect, useState } from "react"
import { Printer as PrinterIcon, RefreshCw, Usb, Wifi } from "lucide-react"
import * as printers from "./printers.ts"
import type { Found, Printer } from "./printers.ts"
import { message } from "./shell.ts"
import { Button, Card, ErrorBanner, Group, Note, Row } from "./ui.tsx"

/** How often the list of set-up printers refreshes. */
const REFRESH_MS = 10_000

export default function Printers() {
  const [list, setList] = useState<Printer[] | null>(null)
  const [found, setFound] = useState<Found[] | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      setList(await printers.listPrinters())
    } catch (e) {
      setError(message(e))
    }
  }, [])

  useEffect(() => {
    void refresh()
    const timer = setInterval(() => void refresh(), REFRESH_MS)
    return () => clearInterval(timer)
  }, [refresh])

  async function act(label: string, change: () => Promise<unknown>, done?: string) {
    setBusy(label)
    setError(null)
    setNotice(null)
    try {
      await change()
      if (done) setNotice(done)
    } catch (e) {
      setError(message(e))
    }
    setBusy(null)
    await refresh()
  }

  const search = () =>
    act("search", async () => setFound(await printers.detect(list ?? [])))

  const add = (f: Found) =>
    act(
      f.uri,
      async () => {
        await printers.add(f, list ?? [])
        setFound((all) => all?.filter((x) => x.uri !== f.uri) ?? null)
      },
      `${f.name} is ready.`,
    )

  if (list === null) return error ? <ErrorBanner>{error}</ErrorBanner> : null

  return (
    <div className="max-w-[460px]">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      {notice && <p className="mb-3 text-[13px] text-accent">{notice}</p>}

      <Group title="My printers">
        <Card>
          {list.length === 0 && <li className="px-4 py-3 text-[13px] text-dim">No printers set up.</li>}
          {list.map((p, i) => (
            <Row key={p.name} first={i === 0}>
              <PrinterIcon size={20} strokeWidth={1.6} />
              <div className="min-w-0 flex-1">
                <div className="truncate text-[14px] font-medium">{p.name.replaceAll("_", " ")}</div>
                <div className="text-[12px] text-dim">
                  {p.state === "idle" ? "Ready" : p.state === "printing" ? "Printing" : "Off"}
                  {p.isDefault ? " · Default" : ""}
                </div>
              </div>
              <Button disabled={busy !== null} onClick={() => void act(`test:${p.name}`, () => printers.testPage(p.name), "Test page sent.")}>
                Test page
              </Button>
              {!p.isDefault && (
                <Button disabled={busy !== null} onClick={() => void act(`def:${p.name}`, () => printers.makeDefault(p.name))}>
                  Make default
                </Button>
              )}
              <Button disabled={busy !== null} onClick={() => void act(`rm:${p.name}`, () => printers.remove(p.name))}>
                Remove
              </Button>
            </Row>
          ))}
        </Card>
      </Group>

      <Group title="Add a printer">
        <div className="mb-2">
          <Button disabled={busy !== null} onClick={() => void search()}>
            <span className="inline-flex items-center gap-1.5">
              <RefreshCw size={12} className={busy === "search" ? "animate-spin" : ""} />
              {busy === "search" ? "Searching…" : "Search for printers"}
            </span>
          </Button>
        </div>
        {found !== null && (
          <Card>
            {found.length === 0 && (
              <li className="px-4 py-3 text-[13px] text-dim">
                No new printers found. Check that the printer is on and on the same network, or
                connected by USB.
              </li>
            )}
            {found.map((f, i) => (
              <Row key={f.uri} first={i === 0}>
                {f.kind === "usb" ? <Usb size={20} strokeWidth={1.6} /> : <Wifi size={20} strokeWidth={1.6} />}
                <div className="min-w-0 flex-1">
                  <div className="truncate text-[14px] font-medium">{f.name}</div>
                  <div className="text-[12px] text-dim">{f.kind === "usb" ? "USB" : "Network"}</div>
                </div>
                <Button primary disabled={busy !== null} onClick={() => void add(f)}>
                  {busy === f.uri ? "Adding…" : "Add"}
                </Button>
              </Row>
            ))}
          </Card>
        )}
        <Note>Printers that work without a driver (most made since 2015) are set up automatically.</Note>
      </Group>
    </div>
  )
}
