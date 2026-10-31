import { useCallback, useEffect, useState } from "react"
import * as orus from "./orus.ts"
import type { Installed, Offer, Source } from "./orus.ts"
import { message } from "./shell.ts"

/** What the store shows, and the one place that changes it. */
export function useStore() {
  const [offers, setOffers] = useState<Offer[]>([])
  const [apps, setApps] = useState<Installed[]>([])
  const [sources, setSources] = useState<Source[]>([])
  const [error, setError] = useState<string | null>(null)
  const [notice, setNotice] = useState<string | null>(null)
  const [busy, setBusy] = useState(false)

  const reload = useCallback(async () => {
    try {
      const [o, i, s] = await Promise.all([orus.search(""), orus.installed(), orus.sources()])
      setOffers(o)
      setApps(i.filter((app) => !app.system))
      setSources(s)
    } catch (e) {
      setError(message(e))
    }
  }, [])

  useEffect(() => {
    void reload()
  }, [reload])

  /** Run one change, show its error or `done`, and read everything again. */
  const act = useCallback(
    async (change: () => Promise<unknown>, done?: string): Promise<boolean> => {
      setBusy(true)
      setError(null)
      setNotice(null)
      let ok = true
      try {
        await change()
        if (done) setNotice(done)
      } catch (e) {
        setError(message(e))
        ok = false
      }
      await reload()
      setBusy(false)
      return ok
    },
    [reload],
  )

  /** Read the catalogs again; a source that fails is named. */
  const checkForUpdates = useCallback(
    () =>
      act(async () => {
        const failed = await orus.refresh()
        if (failed.length > 0) throw new Error(failed.join("\n"))
      }, "Catalogs are up to date."),
    [act],
  )

  const clearMessages = useCallback(() => {
    setError(null)
    setNotice(null)
  }, [])

  return { offers, apps, sources, error, notice, busy, act, checkForUpdates, clearMessages }
}

export type Store = ReturnType<typeof useStore>
