import { useCallback, useEffect, useState } from "react"
import { Bluetooth as BluetoothIcon, Headphones, Keyboard, Mouse, RefreshCw } from "lucide-react"
import * as bt from "./bluetooth.ts"
import type { BtDevice, Controller } from "./bluetooth.ts"
import { message } from "./shell.ts"
import { Button, Card, ErrorBanner, Group, Note, Row, Switch } from "./ui.tsx"

/** How often the list refreshes while the page is open. */
const REFRESH_MS = 8_000
const SCAN_SECONDS = 8

function DeviceIcon({ icon }: { icon: string }) {
  const props = { size: 20, strokeWidth: 1.6 }
  if (icon.includes("mouse")) return <Mouse {...props} />
  if (icon.includes("keyboard")) return <Keyboard {...props} />
  if (icon.includes("audio") || icon.includes("headset") || icon.includes("headphone")) {
    return <Headphones {...props} />
  }
  return <BluetoothIcon {...props} />
}

export default function Bluetooth() {
  const [ctl, setCtl] = useState<Controller | null | undefined>(undefined)
  const [devices, setDevices] = useState<BtDevice[]>([])
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      const c = await bt.controller()
      setCtl(c)
      setDevices(c?.powered ? await bt.listDevices() : [])
    } catch (e) {
      setCtl(null)
      setError(message(e))
    }
  }, [])

  useEffect(() => {
    void refresh()
    const timer = setInterval(() => void refresh(), REFRESH_MS)
    return () => clearInterval(timer)
  }, [refresh])

  async function act(label: string, change: () => Promise<unknown>) {
    setBusy(label)
    setError(null)
    try {
      await change()
    } catch (e) {
      setError(message(e))
    }
    setBusy(null)
    await refresh()
  }

  if (ctl === undefined) return null
  if (ctl === null) {
    return (
      <div className="max-w-[460px]">
        {error && <ErrorBanner>{error}</ErrorBanner>}
        <Note>No Bluetooth adapter was found on this computer.</Note>
      </div>
    )
  }

  const paired = devices.filter((d) => d.paired)
  // Most nearby devices (phones, tags, TVs) advertise no name; listing them buries the real ones.
  const nearby = devices.filter((d) => !d.paired && !bt.isUnnamed(d))

  const row = (d: BtDevice, i: number) => (
    <Row key={d.address} first={i === 0}>
      <DeviceIcon icon={d.icon} />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[14px] font-medium">{bt.displayName(d)}</div>
        <div className="text-[12px] text-dim">{d.connected ? "Connected" : d.paired ? "Not connected" : d.address}</div>
      </div>
      {d.paired ? (
        <>
          {d.connected ? (
            <Button disabled={busy !== null} onClick={() => void act(d.address, () => bt.disconnect(d.address))}>
              Disconnect
            </Button>
          ) : (
            <Button primary disabled={busy !== null} onClick={() => void act(d.address, () => bt.connect(d.address))}>
              Connect
            </Button>
          )}
          <Button disabled={busy !== null} onClick={() => void act(d.address, () => bt.forget(d.address))}>
            Forget
          </Button>
        </>
      ) : (
        <Button primary disabled={busy !== null} onClick={() => void act(d.address, () => bt.pair(d.address))}>
          {busy === d.address ? "Pairing…" : "Pair"}
        </Button>
      )}
    </Row>
  )

  return (
    <div className="max-w-[460px]">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <Card>
        <Row first>
          <BluetoothIcon size={22} strokeWidth={1.6} />
          <span className="flex-1 text-[14px] font-medium">Bluetooth</span>
          <Switch on={ctl.powered} label="Bluetooth" disabled={busy !== null} onChange={(on) => void act("power", () => bt.setPower(on))} />
        </Row>
      </Card>

      {ctl.powered && (
        <>
          <Group title="My devices">
            <Card>
              {paired.length === 0 && (
                <li className="px-4 py-3 text-[13px] text-dim">No paired devices.</li>
              )}
              {paired.map(row)}
            </Card>
          </Group>
          <Group title="Nearby">
            <div className="mb-2">
              <Button disabled={busy !== null} onClick={() => void act("scan", () => bt.scan(SCAN_SECONDS))}>
                <span className="inline-flex items-center gap-1.5">
                  <RefreshCw size={12} className={busy === "scan" ? "animate-spin" : ""} />
                  {busy === "scan" ? "Searching…" : "Search for devices"}
                </span>
              </Button>
            </div>
            <Card>
              {nearby.length === 0 && (
                <li className="px-4 py-3 text-[13px] text-dim">
                  Put the device in pairing mode, then search.
                </li>
              )}
              {nearby.map(row)}
            </Card>
          </Group>
          <Note>Devices that ask for a PIN or passkey cannot be paired here yet.</Note>
        </>
      )}
    </div>
  )
}
