import { useState } from "react"
import { Image as ImageIcon, Waves } from "lucide-react"
import { ACCENTS } from "./appearance.ts"
import type { Appearance as AppearanceSetting, Wallpaper } from "./appearance.ts"
import { Group } from "./ui.tsx"

function Pill({ className }: { className: string }) {
  return <div className={`h-[7px] rounded-full ${className}`} />
}

/** A miniature settings window, light or dark. */
function Preview({ dark }: { dark: boolean }) {
  const bar = dark ? "bg-[#1b2024]" : "bg-[#eceef0]"
  const side = dark ? "bg-[#1e2428]" : "bg-[#f3f4f5]"
  const line = dark ? "bg-[#3d444a]" : "bg-[#d6d9dc]"
  const body = dark ? "bg-[#15191c]" : "bg-white"
  return (
    <div className={`flex h-[97px] flex-col overflow-hidden rounded-xl ${body}`}>
      <div className={`flex h-[14px] items-center justify-between px-2 ${bar}`}>
        <Pill className={`w-6 ${line}`} />
        <div className={`size-[6px] rounded-full ${line}`} />
      </div>
      <div className="flex flex-1">
        <div className={`flex w-[50px] flex-col gap-[6px] px-2 pt-3 ${side}`}>
          {[0, 1, 2, 3, 4].map((i) => (
            <Pill key={i} className={line} />
          ))}
        </div>
        <div className="flex-1 p-3">
          <Pill className="w-10 bg-accent" />
        </div>
      </div>
    </div>
  )
}

function ThemeCard({
  label,
  selected,
  dark,
  onSelect,
}: {
  label: string
  selected: boolean
  dark: boolean
  onSelect: () => void
}) {
  return (
    <button
      role="radio"
      aria-checked={selected}
      onClick={onSelect}
      className={`rounded-2xl flex flex-col bg-card p-2 pb-0 text-left ${
        selected ? "border-2 border-accent" : "border border-card-line"
      }`}
    >
      <Preview dark={dark} />
      <div className="flex items-center gap-3 px-2 py-2">
        <span
          className={`flex size-7 items-center justify-center rounded-full ${
            selected ? "bg-accent" : "border-2 border-dim"
          }`}
        >
          {selected && <span className="size-[9px] rounded-full bg-on-accent" />}
        </span>
        <span className="text-[15px]">{label}</span>
      </div>
    </button>
  )
}

const WALLPAPER_CHOICES: { value: Wallpaper | null; label: string }[] = [
  { value: null, label: "Automatic" },
  { value: "dunes", label: "Dunes" },
  { value: "dunes-light", label: "Dunes light" },
]

const WALLPAPER_PREVIEWS: Record<Wallpaper, string> = {
  dunes: "wallpaper-preview.jpg",
  "dunes-light": "wallpaper-preview-light.jpg",
}

export default function Appearance({
  appearance,
  update,
}: {
  appearance: AppearanceSetting
  update: (patch: Partial<AppearanceSetting>) => void
}) {
  const [choosing, setChoosing] = useState(false)

  return (
    <div className="max-w-[374px]">
      <Group title="Theme">
        <div role="radiogroup" className="grid grid-cols-2 gap-[17px]">
          <ThemeCard
            label="Light"
            dark={false}
            selected={appearance.theme === "light"}
            onSelect={() => update({ theme: "light" })}
          />
          <ThemeCard
            label="Dark"
            dark
            selected={appearance.theme === "dark"}
            onSelect={() => update({ theme: "dark" })}
          />
        </div>
      </Group>

      <Group title="Accent color" gap={26}>
        <div role="radiogroup" className="flex gap-3">
          {ACCENTS.map((name) => {
            const selected = appearance.accent === name
            return (
              <span key={name} className="flex size-[38px] items-center justify-center">
                <button
                  role="radio"
                  aria-checked={selected}
                  aria-label={name}
                  onClick={() => update({ accent: name })}
                  className="size-7 rounded-full"
                  style={{
                    background: appearance.swatches[name],
                    outline: selected ? "2px solid var(--accent)" : "none",
                    outlineOffset: 2,
                  }}
                />
              </span>
            )
          })}
        </div>
      </Group>

      <Group title="Wallpaper" gap={19}>
        <button
          aria-label="Choose wallpaper"
          aria-expanded={choosing}
          onClick={() => setChoosing(!choosing)}
          className="relative block h-[115px] w-full overflow-hidden rounded-xl"
        >
          <img
            src={WALLPAPER_PREVIEWS[appearance.effectiveWallpaper]}
            alt=""
            className="size-full object-cover"
          />
          <ImageIcon size={24} strokeWidth={1.6} className="absolute bottom-3 right-3 text-white" />
        </button>
        {choosing && (
          <div role="radiogroup" className="mt-3 flex gap-2">
            {WALLPAPER_CHOICES.map(({ value, label }) => {
              const selected = appearance.wallpaper === value
              return (
                <button
                  key={label}
                  role="radio"
                  aria-checked={selected}
                  onClick={() => update({ wallpaper: value })}
                  className={`rounded-lg px-3 py-1.5 text-[13px] ${
                    selected ? "bg-sel text-text" : "bg-fill text-text hover:bg-fill-strong"
                  }`}
                >
                  {label}
                </button>
              )
            })}
          </div>
        )}
      </Group>

      <div className="mt-[19px] flex items-center gap-3 border-t border-line pt-[22px]">
        <Waves size={24} strokeWidth={1.6} className="text-text" />
        <span className="flex-1 text-[15px]">Reduce motion</span>
        <button
          role="switch"
          aria-checked={appearance.reduceMotion}
          aria-label="Reduce motion"
          onClick={() => update({ reduceMotion: !appearance.reduceMotion })}
          className="flex h-[26px] w-[46px] items-center rounded-full border px-[3px]"
          style={{
            background: appearance.reduceMotion ? "var(--accent)" : "var(--toggle-track)",
            borderColor: appearance.reduceMotion ? "var(--accent)" : "var(--toggle-edge)",
            justifyContent: appearance.reduceMotion ? "flex-end" : "flex-start",
          }}
        >
          <span className="size-5 rounded-full" style={{ background: "var(--toggle-knob)" }} />
        </button>
      </div>
    </div>
  )
}
