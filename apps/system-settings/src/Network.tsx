import { useCallback, useEffect, useState } from "react"
import { Check, Eye, EyeOff, Lock, RefreshCw, Wifi } from "lucide-react"
import * as nm from "./nmcli.ts"
import type { NetDevice, WifiNetwork } from "./nmcli.ts"
import { message } from "./shell.ts"
import { Button, Card, ErrorBanner, Group } from "./ui.tsx"

/** How often the lists refresh while the page is open. */
const REFRESH_MS = 10_000

export default function Network() {
  const [wifiOn, setWifiOn] = useState<boolean | null>(null)
  const [networks, setNetworks] = useState<WifiNetwork[]>([])
  const [devices, setDevices] = useState<NetDevice[]>([])
  const [error, setError] = useState<string | null>(null)
  const [busy, setBusy] = useState<string | null>(null)
  const [asking, setAsking] = useState<string | null>(null)
  const [password, setPassword] = useState("")
  const [showPassword, setShowPassword] = useState(false)

  const refresh = useCallback(async () => {
    try {
      const on = await nm.wifiEnabled()
      setWifiOn(on)
      setDevices(await nm.listDevices())
      setNetworks(on ? await nm.listWifi() : [])
    } catch (e) {
      setError(message(e))
    }
  }, [])

  useEffect(() => {
    void refresh()
    const timer = setInterval(() => void refresh(), REFRESH_MS)
    return () => clearInterval(timer)
  }, [refresh])

  /** Run one change, show its error, and reload the lists. Resolves to whether it worked. */
  async function act(label: string, change: () => Promise<unknown>): Promise<boolean> {
    setBusy(label)
    setError(null)
    let ok = true
    try {
      await change()
    } catch (e) {
      setError(message(e))
      ok = false
    }
    setBusy(null)
    await refresh()
    return ok
  }

  function join(network: WifiNetwork) {
    if (network.security && !network.saved) {
      setAsking(network.ssid)
      setPassword("")
      setShowPassword(false)
      return
    }
    void act(network.ssid, () => nm.connectWifi(network.ssid))
  }

  async function submitPassword(ssid: string) {
    const ok = await act(ssid, async () => {
      try {
        await nm.connectWifi(ssid, password)
      } catch (e) {
        // nmcli saves the network even when the password is wrong, and a saved network
        // is joined without asking again. Drop it so the next try asks for the password.
        await nm.forgetConnection(ssid).catch(() => {})
        throw e
      }
    })
    if (ok) {
      setAsking(null)
      setPassword("")
    }
  }

  const wifiDevice = devices.find((d) => d.type === "wifi")

  return (
    <>
      {error && <ErrorBanner>{error}</ErrorBanner>}

      <Group title="Connections">
      <Card>
        {devices.length === 0 && (
          <li className="px-4 py-3 text-[13px] text-dim">No network devices found.</li>
        )}
        {devices.map((d, i) => (
          <li
            key={d.name}
            className={`flex items-center gap-3 px-4 py-3 ${i > 0 ? "border-t border-card-line" : ""}`}
          >
            <div className="min-w-0 flex-1">
              <div className="text-[14px] font-medium">
                {d.type === "wifi" ? "Wi-Fi" : "Ethernet"} · {d.name}
              </div>
              <div className="truncate text-[12px] text-dim">
                {d.state === "connected"
                  ? `${d.connection}${d.addresses.length ? " · " + d.addresses.join(", ") : ""}`
                  : d.state}
              </div>
            </div>
            {d.state === "connected" && (
              <Button
                onClick={() => void act(d.name, () => nm.disconnectDevice(d.name))}
                disabled={busy !== null}
              >
                Disconnect
              </Button>
            )}
          </li>
        ))}
      </Card>
      </Group>

      {wifiDevice && wifiOn !== null && (
        <>
          <Group title="Wi-Fi">
          <Card>
            <li className="flex items-center gap-3 px-4 py-3">
              <div className="flex-1 text-[14px] font-medium">Wi-Fi</div>
              <Button onClick={() => void refresh()} disabled={busy !== null}>
                <RefreshCw size={12} className="inline" /> Refresh
              </Button>
              <Button
                primary={!wifiOn}
                onClick={() => void act("wifi", () => nm.setWifiEnabled(!wifiOn))}
                disabled={busy !== null}
              >
                {wifiOn ? "Turn off" : "Turn on"}
              </Button>
            </li>
            {networks.map((n) => (
              <li key={n.ssid} className="border-t border-card-line px-4 py-3">
                <div className="flex items-center gap-3">
                  <Wifi size={16} className="shrink-0 text-dim" />
                  <div className="min-w-0 flex-1">
                    <div className="truncate text-[14px]">{n.ssid}</div>
                    <div className="text-[12px] text-dim">
                      {n.signal}%{n.security ? ` · ${n.security}` : " · open"}
                      {n.saved && !n.active ? " · saved" : ""}
                    </div>
                  </div>
                  {n.security && <Lock size={13} className="shrink-0 text-dim" />}
                  {n.active ? (
                    <Check size={16} className="shrink-0 text-accent" />
                  ) : (
                    <>
                      {n.saved && (
                        <Button
                          onClick={() => void act(n.ssid, () => nm.forgetConnection(n.ssid))}
                          disabled={busy !== null}
                        >
                          Forget
                        </Button>
                      )}
                      <Button primary onClick={() => join(n)} disabled={busy !== null}>
                        {busy === n.ssid ? "Connecting…" : "Join"}
                      </Button>
                    </>
                  )}
                </div>
                {asking === n.ssid && (
                  <form
                    className="mt-3 flex gap-2"
                    onSubmit={(e) => {
                      e.preventDefault()
                      if (password && busy === null) void submitPassword(n.ssid)
                    }}
                  >
                    <div className="relative flex-1">
                      <input
                        autoFocus
                        type={showPassword ? "text" : "password"}
                        placeholder="Password"
                        value={password}
                        onChange={(e) => setPassword(e.target.value)}
                        className="w-full rounded-md bg-bg py-1.5 pl-2 pr-9 text-[13px] outline-none ring-1 ring-card-line focus:ring-accent"
                      />
                      <button
                        type="button"
                        aria-label={showPassword ? "Hide password" : "Show password"}
                        aria-pressed={showPassword}
                        onClick={() => setShowPassword((v) => !v)}
                        className="absolute right-1 top-1/2 -translate-y-1/2 rounded p-1.5 text-dim hover:text-text"
                      >
                        {showPassword ? <EyeOff size={15} /> : <Eye size={15} />}
                      </button>
                    </div>
                    <Button primary onClick={() => void submitPassword(n.ssid)} disabled={!password || busy !== null}>
                      Connect
                    </Button>
                    <Button onClick={() => setAsking(null)}>Cancel</Button>
                  </form>
                )}
              </li>
            ))}
            {wifiOn && networks.length === 0 && (
              <li className="border-t border-card-line px-4 py-3 text-[13px] text-dim">
                No networks in range.
              </li>
            )}
          </Card>
          </Group>
        </>
      )}
    </>
  )
}
