import { useCallback, useEffect, useRef, useState } from "react"
import { DEFAULT_APPEARANCE, applyAppearance, readAppearance } from "./appearance.ts"
import type { Appearance } from "./appearance.ts"
import { message } from "./shell.ts"

/** How often the setting is read again, to follow changes made elsewhere. */
const POLL_MS = 3_000

/** Put the setting on the page: the theme, the accent colors and reduced motion. */
function paint(a: Appearance) {
  const root = document.documentElement
  root.dataset.theme = a.theme
  root.dataset.reduceMotion = String(a.reduceMotion)
  root.style.setProperty("--accent", a.colors.accent)
  root.style.setProperty("--sel", a.colors.selection)
}

export function useAppearance() {
  const [appearance, setAppearance] = useState<Appearance>(DEFAULT_APPEARANCE)
  const [error, setError] = useState<string | null>(null)
  const busy = useRef(false)

  useEffect(() => paint(appearance), [appearance])

  useEffect(() => {
    const load = () => {
      if (busy.current) return
      readAppearance()
        .then((a) => {
          setAppearance(a)
          setError(null)
        })
        .catch((e) => setError(message(e)))
    }
    load()
    const timer = setInterval(load, POLL_MS)
    return () => clearInterval(timer)
  }, [])

  const update = useCallback(
    async (patch: Partial<Appearance>) => {
      const prev = appearance
      const next = { ...prev, ...patch }
      setAppearance(next)
      busy.current = true
      try {
        setAppearance(await applyAppearance(prev, next))
        setError(null)
      } catch (e) {
        setAppearance(prev)
        setError(message(e))
      } finally {
        busy.current = false
      }
    },
    [appearance],
  )

  return { appearance, update, error }
}
