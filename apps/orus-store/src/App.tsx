import { useState } from "react"
import { Download, Globe, Package } from "lucide-react"
import type { LucideIcon } from "lucide-react"
import Discover from "./Discover.tsx"
import Installed from "./Installed.tsx"
import Sources from "./Sources.tsx"
import { ErrorBanner } from "./ui.tsx"
import { useAppearance } from "./useAppearance.ts"
import { useStore } from "./useStore.ts"

type PageId = "discover" | "installed" | "sources"

const pages: { id: PageId; title: string; icon: LucideIcon }[] = [
  { id: "discover", title: "Discover", icon: Download },
  { id: "installed", title: "Installed", icon: Package },
  { id: "sources", title: "Sources", icon: Globe },
]

export default function App() {
  const [selected, setSelected] = useState<PageId>("discover")
  const store = useStore()
  useAppearance()
  const current = pages.find((p) => p.id === selected) ?? pages[0]

  return (
    <div className="flex h-full">
      <aside className="w-[187px] shrink-0 border-r border-line bg-side px-3 py-4">
        <nav className="flex flex-col gap-[5px]">
          {pages.map((p) => {
            const Icon = p.icon
            const active = p.id === selected
            return (
              <button
                key={p.id}
                onClick={() => {
                  setSelected(p.id)
                  store.clearMessages()
                }}
                className={`flex h-11 items-center gap-[14px] rounded-lg px-[14px] text-left text-[15px] ${
                  active ? "bg-sel text-text" : "text-text hover:bg-fill-soft"
                }`}
              >
                <Icon size={22} strokeWidth={1.6} className={active ? "text-accent" : ""} />
                {p.title}
              </button>
            )
          })}
        </nav>
      </aside>
      <main className="flex-1 overflow-y-auto bg-bg px-6 py-6">
        <h1 className="mb-3.5 text-[26px] font-semibold leading-8 text-text">{current.title}</h1>
        {store.error && <ErrorBanner>{store.error}</ErrorBanner>}
        {store.notice && <p className="mb-3 text-[13px] text-accent">{store.notice}</p>}
        {selected === "discover" && <Discover store={store} />}
        {selected === "installed" && <Installed store={store} />}
        {selected === "sources" && <Sources store={store} />}
      </main>
    </div>
  )
}
