import { useEffect, useState } from "react"
import { Battery, BatteryCharging } from "lucide-react"
import { message } from "./shell.ts"
import { readPower } from "./system.ts"
import type { BatteryInfo } from "./system.ts"
import { Card, ErrorBanner, Group, Row } from "./ui.tsx"

/** How often the battery is read again. */
const REFRESH_MS = 10_000

export default function Power() {
  const [batteries, setBatteries] = useState<BatteryInfo[] | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    const load = () =>
      readPower()
        .then((b) => {
          setBatteries(b)
          setError(null)
        })
        .catch((e) => setError(message(e)))
    void load()
    const timer = setInterval(() => void load(), REFRESH_MS)
    return () => clearInterval(timer)
  }, [])

  return (
    <div className="max-w-[374px]">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <Group title="Battery">
        <Card>
          {batteries?.length === 0 && (
            <Row first>
              <span className="text-[13px] text-dim">No battery detected.</span>
            </Row>
          )}
          {batteries?.map((b, i) => (
            <Row key={b.name} first={i === 0}>
              {b.status === "Charging" ? (
                <BatteryCharging size={22} strokeWidth={1.6} />
              ) : (
                <Battery size={22} strokeWidth={1.6} />
              )}
              <div className="min-w-0 flex-1">
                <div className="text-[14px] font-medium">
                  {b.percent}% · {b.status}
                </div>
                <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-fill">
                  <div className="h-full rounded-full bg-accent" style={{ width: `${b.percent}%` }} />
                </div>
              </div>
            </Row>
          ))}
        </Card>
      </Group>
    </div>
  )
}
