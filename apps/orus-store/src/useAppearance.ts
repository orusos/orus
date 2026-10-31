import { useEffect, useState } from "react"
import { DEFAULT_APPEARANCE, readAppearance } from "./appearance.ts"
import type { Appearance } from "./appearance.ts"

/** How often the setting is read again, to follow changes made in Settings. */
const POLL_MS = 3_000

/** Put the setting on the page: the theme, the accent colors and reduced motion. */
function paint(a: Appearance) {
  const root = document.documentElement
  root.dataset.theme = a.theme
  root.dataset.reduceMotion = String(a.reduceMotion)
  root.style.setProperty("--accent", a.colors.accent)
  root.style.setProperty("--sel", a.colors.selection)
}

/** Follow the shared theme and accent. Without `orus-appearance` the page keeps its default. */
export function useAppearance() {
  const [appearance, setAppearance] = useState<Appearance>(DEFAULT_APPEARANCE)

  useEffect(() => paint(appearance), [appearance])

  useEffect(() => {
    const load = () => readAppearance().then(setAppearance, () => {})
    void load()
    const timer = setInterval(load, POLL_MS)
    return () => clearInterval(timer)
  }, [])
}
