import { useState } from "react"
import {
  BatteryMedium,
  Bluetooth as BluetoothIcon,
  Info,
  Monitor,
  Mouse,
  Printer,
  Paintbrush,
  Shield,
  Users as UsersIcon,
  Volume2,
  Wifi,
} from "lucide-react"
import type { LucideIcon } from "lucide-react"
import About from "./About.tsx"
import Bluetooth from "./Bluetooth.tsx"
import Display from "./Display.tsx"
import Pointer from "./Pointer.tsx"
import Printers from "./Printers.tsx"
import Sound from "./Sound.tsx"
import Users from "./Users.tsx"
import Appearance from "./Appearance.tsx"
import type { Appearance as AppearanceSetting } from "./appearance.ts"
import type { SectionId } from "./data.ts"
import Network from "./Network.tsx"
import Power from "./Power.tsx"
import Privacy from "./Privacy.tsx"
import { ErrorBanner, NotBuilt } from "./ui.tsx"
import { useAppearance } from "./useAppearance.ts"

interface Section {
  id: SectionId
  title: string
  icon: LucideIcon
}

const sections: Section[] = [
  { id: "appearance", title: "Appearance", icon: Paintbrush },
  { id: "network", title: "Network", icon: Wifi },
  { id: "bluetooth", title: "Bluetooth", icon: BluetoothIcon },
  { id: "display", title: "Display", icon: Monitor },
  { id: "sound", title: "Sound", icon: Volume2 },
  { id: "pointer", title: "Mouse & Touchpad", icon: Mouse },
  { id: "printers", title: "Printers", icon: Printer },
  { id: "power", title: "Power", icon: BatteryMedium },
  { id: "users", title: "Users & Groups", icon: UsersIcon },
  { id: "privacy", title: "Privacy", icon: Shield },
  { id: "about", title: "About", icon: Info },
]

function Page({
  id,
  title,
  appearance,
  update,
}: {
  id: SectionId
  title: string
  appearance: AppearanceSetting
  update: (patch: Partial<AppearanceSetting>) => void
}) {
  switch (id) {
    case "appearance":
      return <Appearance appearance={appearance} update={update} />
    case "network":
      return <Network />
    case "bluetooth":
      return <Bluetooth />
    case "display":
      return <Display />
    case "sound":
      return <Sound />
    case "pointer":
      return <Pointer />
    case "printers":
      return <Printers />
    case "power":
      return <Power />
    case "users":
      return <Users />
    case "privacy":
      return <Privacy />
    case "about":
      return <About />
    default:
      return <NotBuilt what={title} />
  }
}

export default function App() {
  const [selected, setSelected] = useState<SectionId>("appearance")
  const current = sections.find((s) => s.id === selected) ?? sections[0]
  const { appearance, update, error } = useAppearance()

  return (
    <div className="flex h-full">
      <aside className="w-[187px] shrink-0 border-r border-line bg-side px-3 py-4">
        <nav className="flex flex-col gap-[5px]">
          {sections.map((s) => {
            const Icon = s.icon
            const active = s.id === selected
            return (
              <button
                key={s.id}
                onClick={() => setSelected(s.id)}
                className={`flex h-11 items-center gap-[14px] rounded-lg px-[14px] text-left text-[15px] ${
                  active ? "bg-sel text-text" : "text-text hover:bg-fill-soft"
                }`}
              >
                <Icon
                  size={22}
                  strokeWidth={1.6}
                  className={`${active ? "text-accent" : ""} ${s.id === "power" ? "-rotate-90" : ""}`}
                />
                {s.title}
              </button>
            )
          })}
        </nav>
      </aside>
      <main className="flex-1 overflow-y-auto bg-bg px-6 py-6">
        <h1 className="mb-3.5 text-[26px] font-semibold leading-8 text-text">{current.title}</h1>
        {error && <ErrorBanner>{error}</ErrorBanner>}
        <div>
          <Page id={current.id} title={current.title} appearance={appearance} update={update} />
        </div>
      </main>
    </div>
  )
}
