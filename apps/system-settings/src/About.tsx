import { useEffect, useState } from "react"
import { message } from "./shell.ts"
import { readAbout } from "./system.ts"
import type { AboutInfo } from "./system.ts"
import { Card, ErrorBanner, Row } from "./ui.tsx"

export default function About() {
  const [info, setInfo] = useState<AboutInfo | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    readAbout()
      .then(setInfo)
      .catch((e) => setError(message(e)))
  }, [])

  const rows: [string, string][] = info
    ? [
        ["Device name", info.hostname],
        ["Orus OS", info.version],
        ["Base system", info.os],
        ["Kernel", info.kernel],
        ["Architecture", info.arch],
        ["Memory", info.memoryGiB && `${info.memoryGiB} GiB`],
      ].filter((r): r is [string, string] => r[1] !== "")
    : []

  return (
    <div className="max-w-[374px]">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <Card>
        {rows.map(([label, value], i) => (
          <Row key={label} first={i === 0}>
            <span className="w-28 shrink-0 text-[13px] text-dim">{label}</span>
            <span className="min-w-0 flex-1 truncate text-[14px]">{value}</span>
          </Row>
        ))}
      </Card>
    </div>
  )
}
