/**
 * Users and groups. Reading needs no privilege. Every change opens a Terminal that runs
 * `sudo ...`, as the Office install does, so passwords are typed to sudo and passwd and
 * never pass through this app.
 */

import { run } from "./shell.ts"

export interface User {
  name: string
  fullName: string
  uid: number
  groups: string[]
  admin: boolean
}

/** Groups a new user gets, as the installer's `users.conf` gives them (without `sudo`). */
const DEFAULT_GROUPS = "cdrom,floppy,audio,dip,video,plugdev,netdev,lpadmin,scanner,bluetooth"

const NAME = /^[a-z][a-z0-9_-]{0,31}$/

export const validName = (name: string) => NAME.test(name)

/** People, not services: uid 1000 up with a login shell. */
export function parseUsers(passwd: string, group: string): User[] {
  const groups = group
    .split("\n")
    .map((l) => l.split(":"))
    .filter((f) => f.length >= 4)
    .map((f) => ({ name: f[0], gid: f[2], members: f[3] ? f[3].split(",") : [] }))
  const users: User[] = []
  for (const line of passwd.split("\n")) {
    const f = line.split(":")
    if (f.length < 7) continue
    const uid = Number(f[2])
    if (uid < 1000 || uid >= 60000 || /(nologin|false)$/.test(f[6])) continue
    const own = groups.filter((g) => g.gid === f[3] || g.members.includes(f[0])).map((g) => g.name)
    users.push({
      name: f[0],
      fullName: f[4].split(",")[0],
      uid,
      groups: own.sort(),
      admin: own.includes("sudo"),
    })
  }
  return users.sort((a, b) => a.uid - b.uid)
}

export async function listUsers(): Promise<{ users: User[]; me: string }> {
  const [passwd, group, me] = await Promise.all([
    run("getent", ["passwd"]),
    run("getent", ["group"]),
    run("id", ["-un"]),
  ])
  return { users: parseUsers(passwd, group), me: me.trim() }
}

/** `text` as one shell word. */
const quote = (text: string) => `'${text.replaceAll("'", "'\\''")}'`

/** The Terminal command that adds `name`; throws if `name` is not a valid login name. */
export function addCommand(name: string, fullName: string, admin: boolean): string {
  if (!validName(name)) throw new Error("A user name is lowercase letters, digits, - and _, and starts with a letter.")
  const groups = admin ? `${DEFAULT_GROUPS},sudo` : DEFAULT_GROUPS
  const comment = fullName.replace(/[:,\n\r]/g, " ").trim() || name
  return `sudo adduser --comment ${quote(comment)} ${name} && sudo usermod -aG ${groups} ${name}`
}

export const passwordCommand = (name: string) => `sudo passwd ${name}`
export const adminCommand = (name: string, admin: boolean) =>
  admin ? `sudo usermod -aG sudo ${name}` : `sudo gpasswd -d ${name} sudo`
export const removeCommand = (name: string) => `sudo deluser --remove-home ${name}`

/** Open a Terminal window that runs `command`; sudo asks for the password there. */
export async function inTerminal(command: string): Promise<void> {
  await run("sh", [
    "-c",
    'ORUS_TERMINAL_COMMAND="$1" orus-linux --app terminal --size 760x420 >/dev/null 2>&1 &',
    "sh",
    command,
  ])
}
