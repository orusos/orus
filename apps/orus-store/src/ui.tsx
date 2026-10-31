import type { ReactNode } from "react"

export function Card({ children }: { children: ReactNode }) {
  return (
    <ul className="overflow-hidden rounded-2xl border border-card-line bg-card">{children}</ul>
  )
}

export function Row({ children, first }: { children: ReactNode; first?: boolean }) {
  return (
    <li className={`flex items-center gap-3 px-4 py-3 ${first ? "" : "border-t border-card-line"}`}>
      {children}
    </li>
  )
}

export function Empty({ children }: { children: ReactNode }) {
  return (
    <div className="rounded-2xl border border-card-line bg-card px-4 py-10 text-center text-[13px] text-dim">
      {children}
    </div>
  )
}

export function ErrorBanner({ children }: { children: ReactNode }) {
  return (
    <div className="mb-3 rounded-xl border border-warn/40 bg-warn/10 px-3 py-2 text-[13px] text-warn">
      {children}
    </div>
  )
}

export function Button({
  children,
  onClick,
  disabled,
  primary,
  danger,
}: {
  children: ReactNode
  onClick: () => void
  disabled?: boolean
  primary?: boolean
  danger?: boolean
}) {
  const look = primary
    ? "bg-accent text-on-accent"
    : danger
      ? "bg-fill text-warn hover:bg-fill-strong"
      : "bg-fill text-text hover:bg-fill-strong"
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`rounded-lg px-3 py-1 text-[12px] font-medium disabled:opacity-40 ${look}`}
    >
      {children}
    </button>
  )
}

export function Badge({ children, warn }: { children: ReactNode; warn?: boolean }) {
  return (
    <span
      className={`rounded-md px-1.5 py-0.5 text-[11px] ${warn ? "bg-warn/10 text-warn" : "bg-fill text-dim"}`}
    >
      {children}
    </span>
  )
}

/** A box over the page that asks one question. */
export function Dialog({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="fixed inset-0 z-10 flex items-center justify-center bg-black/50 p-6">
      <div className="w-full max-w-[420px] rounded-2xl border border-card-line bg-card p-5">
        <h2 className="mb-3 text-[17px] font-semibold text-text">{title}</h2>
        {children}
      </div>
    </div>
  )
}
