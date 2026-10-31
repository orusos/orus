import { useEffect, useState } from "react"
import type { ReactNode } from "react"

/** A titled group of controls, with the concept's label above it. */
export function Group({
  title,
  children,
  gap = 24,
}: {
  title: string
  children: ReactNode
  /** Space above the group, in px; the concept spaces its groups unevenly. */
  gap?: number
}) {
  return (
    <section className="first:!mt-0" style={{ marginTop: gap }}>
      <h2 className="mb-2.5 text-[15px] font-semibold leading-5 text-text">{title}</h2>
      {children}
    </section>
  )
}

export function Card({ children }: { children: ReactNode }) {
  return (
    <ul className="rounded-2xl border border-card-line bg-card">{children}</ul>
  )
}

export function Row({ children, first }: { children: ReactNode; first?: boolean }) {
  return (
    <li className={`flex items-center gap-3 px-4 py-3 ${first ? "" : "border-t border-card-line"}`}>
      {children}
    </li>
  )
}

export function Note({ children }: { children: ReactNode }) {
  return <p className="mb-5 text-[13px] text-dim">{children}</p>
}

export function NotBuilt({ what }: { what: string }) {
  return (
    <div className="rounded-2xl border border-card-line bg-card px-4 py-10 text-center text-[13px] text-dim">
      {what} settings are not available yet.
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
}: {
  children: ReactNode
  onClick: () => void
  disabled?: boolean
  primary?: boolean
}) {
  return (
    <button
      onClick={onClick}
      disabled={disabled}
      className={`rounded-lg px-3 py-1 text-[12px] font-medium disabled:opacity-40 ${
        primary ? "bg-accent text-on-accent" : "bg-fill text-text hover:bg-fill-strong"
      }`}
    >
      {children}
    </button>
  )
}

export function Switch({
  on,
  label,
  onChange,
  disabled,
}: {
  on: boolean
  label: string
  onChange: (on: boolean) => void
  disabled?: boolean
}) {
  return (
    <button
      role="switch"
      aria-checked={on}
      aria-label={label}
      disabled={disabled}
      onClick={() => onChange(!on)}
      className="flex h-[26px] w-[46px] shrink-0 items-center rounded-full border px-[3px] disabled:opacity-40"
      style={{
        background: on ? "var(--accent)" : "var(--toggle-track)",
        borderColor: on ? "var(--accent)" : "var(--toggle-edge)",
        justifyContent: on ? "flex-end" : "flex-start",
      }}
    >
      <span className="size-5 rounded-full" style={{ background: "var(--toggle-knob)" }} />
    </button>
  )
}

/** A card row with a label and a switch. */
export function SwitchRow({
  label,
  on,
  onChange,
  first,
  disabled,
}: {
  label: string
  on: boolean
  onChange: (on: boolean) => void
  first?: boolean
  disabled?: boolean
}) {
  return (
    <Row first={first}>
      <span className="flex-1 text-[14px]">{label}</span>
      <Switch on={on} label={label} onChange={onChange} disabled={disabled} />
    </Row>
  )
}

/** A card row with a label and a slider; `onChange` fires when the user lets go. */
export function SliderRow({
  label,
  value,
  min,
  max,
  step,
  format,
  onChange,
  first,
}: {
  label: string
  value: number
  min: number
  max: number
  step: number
  format: (value: number) => string
  onChange: (value: number) => void
  first?: boolean
}) {
  const [shown, setShown] = useState(value)
  useEffect(() => setShown(value), [value])
  return (
    <Row first={first}>
      <span className="w-[120px] shrink-0 text-[14px]">{label}</span>
      <input
        type="range"
        aria-label={label}
        min={min}
        max={max}
        step={step}
        value={shown}
        onChange={(e) => setShown(Number(e.target.value))}
        onPointerUp={() => onChange(shown)}
        onKeyUp={() => onChange(shown)}
        className="flex-1 accent-accent"
      />
      <span className="w-12 text-right text-[13px] text-dim">{format(shown)}</span>
    </Row>
  )
}

export interface Choice {
  value: string
  label: string
}

/**
 * A dropdown drawn by the page. The runtime draws a native `<select>` list on a light
 * background of its own and ignores the options' colors, so the page's light text
 * cannot be read there.
 */
export function Select({
  label,
  value,
  choices,
  onChange,
  disabled,
  placeholder,
}: {
  label: string
  value: string
  choices: Choice[]
  onChange: (value: string) => void
  disabled?: boolean
  /** Shown when `value` is none of the choices. */
  placeholder?: string
}) {
  const [open, setOpen] = useState(false)

  const current = choices.find((c) => c.value === value)
  return (
    <div className="relative" onKeyDown={(e) => e.key === "Escape" && setOpen(false)}>
      <button
        aria-label={label}
        aria-haspopup="listbox"
        aria-expanded={open}
        disabled={disabled}
        onClick={() => setOpen(!open)}
        className="flex min-w-[150px] items-center justify-between gap-3 rounded-lg bg-fill px-3 py-1 text-[13px] text-text disabled:opacity-40"
      >
        <span>{current?.label ?? placeholder ?? ""}</span>
        <span aria-hidden className="text-dim">
          {open ? "▴" : "▾"}
        </span>
      </button>
      {open && <div className="fixed inset-0 z-10" onClick={() => setOpen(false)} />}
      {open && (
        <ul
          role="listbox"
          aria-label={label}
          className="absolute right-0 top-full z-20 mt-1 max-h-[260px] min-w-full overflow-y-auto rounded-lg border border-card-line bg-card py-1 shadow-lg"
        >
          {choices.map((c) => (
            <li
              key={c.value}
              role="option"
              aria-selected={c.value === value}
              onClick={() => {
                setOpen(false)
                if (c.value !== value) onChange(c.value)
              }}
              className={`cursor-default whitespace-nowrap px-3 py-1.5 text-[13px] ${
                c.value === value ? "bg-sel text-text" : "text-text hover:bg-fill-soft"
              }`}
            >
              {c.label}
            </li>
          ))}
        </ul>
      )}
    </div>
  )
}
