export type SectionId =
  | "appearance"
  | "network"
  | "bluetooth"
  | "display"
  | "sound"
  | "pointer"
  | "printers"
  | "power"
  | "users"
  | "privacy"
  | "about"

export interface App {
  symbol: string
  name: string
  description: string
  color: string
  /** Access the app declares, shown as small labels. */
  access: string[]
}

/** The apps the shell ships, with the access each one declares. */
export const apps: App[] = [
  {
    symbol: ">_",
    name: "Terminal",
    description: "System shell with PTY access",
    color: "#22c55e",
    access: ["filesystem", "no network"],
  },
  {
    symbol: "Aa",
    name: "Text Edit",
    description: "Plain text and markdown editor",
    color: "#f59e0b",
    access: ["filesystem"],
  },
  {
    symbol: "FM",
    name: "Files",
    description: "Sandboxed file browser for ~/.orus/",
    color: "#06b6d4",
    access: ["filesystem"],
  },
]
