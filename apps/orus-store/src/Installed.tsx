import { useState } from "react"
import * as orus from "./orus.ts"
import type { Installed as InstalledApp, Offer } from "./orus.ts"
import { isNewer } from "./orus.ts"
import Review from "./Review.tsx"
import type { Store } from "./useStore.ts"
import { Badge, Button, Card, Dialog, Empty, Row } from "./ui.tsx"

export default function Installed({ store }: { store: Store }) {
  const [reviewing, setReviewing] = useState<Offer | null>(null)
  const [removing, setRemoving] = useState<InstalledApp | null>(null)

  /** The newest version any source offers, when it is newer than the installed one. */
  const update = (app: InstalledApp): Offer | undefined =>
    store.offers
      .filter((o) => o.id === app.id && isNewer(o.version, app.version))
      .sort((a, b) => (isNewer(a.version, b.version) ? -1 : 1))[0]

  async function install(offer: Offer) {
    const ok = await store.act(() => orus.install(offer), `${offer.name} is updated.`)
    if (ok) setReviewing(null)
  }

  async function uninstall(app: InstalledApp, withData: boolean) {
    const change = withData ? orus.purge : orus.remove
    await store.act(() => change(app.id), `${app.name} is removed.`)
    setRemoving(null)
  }

  return (
    <>
      <div className="mb-4">
        <Button onClick={() => void store.checkForUpdates()} disabled={store.busy}>
          Check for updates
        </Button>
      </div>

      {store.apps.length === 0 ? (
        <Empty>No apps installed from the store yet.</Empty>
      ) : (
        <Card>
          {store.apps.map((app, i) => {
            const newer = update(app)
            return (
              <Row key={app.id} first={i === 0}>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center gap-2">
                    <span className="text-[14px] font-medium text-text">{app.name}</span>
                    <span className="text-[12px] text-dim">{app.version}</span>
                    {newer && <Badge>{newer.version} available</Badge>}
                  </div>
                  <p className="truncate text-[13px] text-dim">{app.description || app.id}</p>
                </div>
                {newer && (
                  <Button primary onClick={() => setReviewing(newer)} disabled={store.busy}>
                    Update
                  </Button>
                )}
                <Button onClick={() => void store.act(() => orus.open(app.id))} disabled={store.busy}>
                  Open
                </Button>
                {app.canRollBack && (
                  <Button
                    onClick={() => void store.act(() => orus.rollback(app.id), `${app.name} went back a version.`)}
                    disabled={store.busy}
                  >
                    Roll back
                  </Button>
                )}
                <Button danger onClick={() => setRemoving(app)} disabled={store.busy}>
                  Remove
                </Button>
              </Row>
            )
          })}
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

      {removing && (
        <Dialog title={`Remove ${removing.name}?`}>
          <p className="mb-4 text-[13px] text-dim">
            Remove keeps what the app saved, so a reinstall finds it again. Remove and delete data
            also deletes what it saved here: its storage and keychain entries.
          </p>
          <div className="flex justify-end gap-2">
            <Button onClick={() => setRemoving(null)} disabled={store.busy}>
              Cancel
            </Button>
            <Button onClick={() => void uninstall(removing, false)} disabled={store.busy}>
              Remove
            </Button>
            <Button danger onClick={() => void uninstall(removing, true)} disabled={store.busy}>
              Remove and delete data
            </Button>
          </div>
        </Dialog>
      )}
    </>
  )
}
