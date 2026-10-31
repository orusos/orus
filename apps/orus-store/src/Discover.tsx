import { useMemo, useState } from "react"
import { Search } from "lucide-react"
import * as orus from "./orus.ts"
import type { Offer } from "./orus.ts"
import Review from "./Review.tsx"
import type { Store } from "./useStore.ts"
import { Badge, Button, Card, Empty, Row } from "./ui.tsx"

export default function Discover({ store }: { store: Store }) {
  const [query, setQuery] = useState("")
  const [reviewing, setReviewing] = useState<Offer | null>(null)

  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return store.offers.filter((o) =>
      [o.id, o.name, o.description].some((f) => f.toLowerCase().includes(needle)),
    )
  }, [store.offers, query])

  async function install(offer: Offer) {
    const ok = await store.act(() => orus.install(offer), `${offer.name} is installed.`)
    if (ok) setReviewing(null)
  }

  return (
    <>
      <label className="mb-4 flex items-center gap-2 rounded-xl border border-card-line bg-card px-3 py-2">
        <Search size={16} className="text-dim" />
        <input
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Search apps"
          className="flex-1 bg-transparent text-[14px] text-text outline-none placeholder:text-dim"
        />
      </label>

      {shown.length === 0 ? (
        <Empty>
          {store.offers.length === 0
            ? "No apps yet. Add a source, then check for updates."
            : "No app matches your search."}
        </Empty>
      ) : (
        <Card>
          {shown.map((o, i) => (
            <Row key={`${o.source}/${o.id}`} first={i === 0}>
              <div className="min-w-0 flex-1">
                <div className="flex items-center gap-2">
                  <span className="text-[14px] font-medium text-text">{o.name}</span>
                  <span className="text-[12px] text-dim">{o.version}</span>
                  {o.unsigned && <Badge warn>unsigned</Badge>}
                </div>
                <p className="truncate text-[13px] text-dim">{o.description || o.id}</p>
              </div>
              {o.installed === o.version ? (
                <Button onClick={() => void store.act(() => orus.open(o.id))} disabled={store.busy}>
                  Open
                </Button>
              ) : (
                <Button primary onClick={() => setReviewing(o)} disabled={store.busy}>
                  {o.installed ? "Update" : "Get"}
                </Button>
              )}
            </Row>
          ))}
        </Card>
      )}

      {reviewing && (
        <Review
          offer={reviewing}
          busy={store.busy}
          onInstall={() => void install(reviewing)}
          onCancel={() => setReviewing(null)}
        />
      )}
    </>
  )
}
