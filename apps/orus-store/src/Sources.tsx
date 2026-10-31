import { useState } from "react"
import * as orus from "./orus.ts"
import type { Store } from "./useStore.ts"
import { Badge, Button, Card, Empty, Row } from "./ui.tsx"

export default function Sources({ store }: { store: Store }) {
  const [url, setUrl] = useState("")
  const [development, setDevelopment] = useState(false)
  const [key, setKey] = useState("")

  const canAdd = url.trim() !== "" && (development || key.trim() !== "")

  async function add() {
    const ok = await store.act(
      async () => {
        await orus.addSource(url.trim(), development, development ? null : key.trim())
        await orus.refresh()
      },
      "Source added.",
    )
    if (ok) {
      setUrl("")
      setKey("")
    }
  }

  return (
    <>
      {store.sources.length === 0 ? (
        <Empty>No sources.</Empty>
      ) : (
        <Card>
          {store.sources.map((s, i) => (
            <Row key={s.name} first={i === 0}>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-[14px] font-medium text-text">{s.name}</span>
                  {s.development && <Badge warn>development, unsigned</Badge>}
                  {s.system && <Badge>system</Badge>}
                  {!s.enabled && <Badge>off</Badge>}
                </div>
                <p className="truncate text-[13px] text-dim">{s.url}</p>
              </div>
              <Button
                onClick={() => void store.act(() => orus.setSourceEnabled(s.name, !s.enabled))}
                disabled={store.busy}
              >
                {s.enabled ? "Turn off" : "Turn on"}
              </Button>
              {!s.system && (
                <Button
                  danger
                  onClick={() => void store.act(() => orus.removeSource(s.name))}
                  disabled={store.busy}
                >
                  Remove
                </Button>
              )}
            </Row>
          ))}
        </Card>
      )}

      <h2 className="mb-2.5 mt-6 text-[15px] font-semibold text-text">Add a source</h2>
      <div className="flex flex-col gap-2 rounded-2xl border border-card-line bg-card p-4">
        <input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder={development ? "http://192.168.64.1:6678" : "https://packages.example.com"}
          className="rounded-lg bg-fill px-3 py-2 text-[13px] text-text outline-none placeholder:text-dim"
        />
        <label className="flex items-center gap-2 text-[13px] text-text">
          <input
            type="checkbox"
            checked={development}
            onChange={(e) => setDevelopment(e.target.checked)}
          />
          Development source (not signed, private network only)
        </label>
        {!development && (
          <input
            value={key}
            onChange={(e) => setKey(e.target.value)}
            placeholder="Path to the source's public key file"
            className="rounded-lg bg-fill px-3 py-2 text-[13px] text-text outline-none placeholder:text-dim"
          />
        )}
        <div>
          <Button primary onClick={() => void add()} disabled={!canAdd || store.busy}>
            Add
          </Button>
        </div>
      </div>
    </>
  )
}
