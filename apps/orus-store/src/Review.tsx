import { ShieldAlert } from "lucide-react"
import { capabilityLabel, formatSize } from "./orus.ts"
import type { Offer } from "./orus.ts"
import { Button, Dialog } from "./ui.tsx"

/** What installing `offer` does, shown before anything is downloaded. */
export default function Review({
  offer,
  busy,
  onInstall,
  onCancel,
}: {
  offer: Offer
  busy: boolean
  onInstall: () => void
  onCancel: () => void
}) {
  const verb = offer.installed ? "Update" : "Install"
  return (
    <Dialog title={`${verb} ${offer.name}?`}>
      <p className="mb-1 text-[13px] text-dim">
        Version {offer.version} · {formatSize(offer.size)} · from {offer.source}
      </p>
      {offer.unsigned && (
        <p className="mb-3 flex items-start gap-2 text-[13px] text-warn">
          <ShieldAlert size={16} className="mt-0.5 shrink-0" />
          <span>This source is not signed. Install only what you trust.</span>
        </p>
      )}
      {offer.capabilities.length === 0 ? (
        <p className="mb-4 text-[13px] text-text">It asks for no extra permissions.</p>
      ) : (
        <>
          <p className="mb-1 text-[13px] text-text">It asks to:</p>
          <ul className="mb-4 list-disc pl-5 text-[13px] text-text">
            {offer.capabilities.map((c) => (
              <li key={c}>
                {capabilityLabel(c)} <span className="text-dim">({c})</span>
              </li>
            ))}
          </ul>
        </>
      )}
      <div className="flex justify-end gap-2">
        <Button onClick={onCancel} disabled={busy}>
          Cancel
        </Button>
        <Button primary onClick={onInstall} disabled={busy}>
          {busy ? "Working…" : verb}
        </Button>
      </div>
    </Dialog>
  )
}
