import { useCallback, useEffect, useState } from "react"
import { UserRound } from "lucide-react"
import { message } from "./shell.ts"
import * as users from "./users.ts"
import type { User } from "./users.ts"
import { Button, Card, ErrorBanner, Group, Note, Row, Switch } from "./ui.tsx"

/** How often the list refreshes, so a change made in the Terminal shows up. */
const REFRESH_MS = 4_000

const inputClass =
  "rounded-lg bg-fill px-3 py-1 text-[13px] text-text outline-none placeholder:text-dim"

export default function Users() {
  const [list, setList] = useState<User[]>([])
  const [me, setMe] = useState("")
  const [error, setError] = useState<string | null>(null)
  const [adding, setAdding] = useState(false)
  const [name, setName] = useState("")
  const [fullName, setFullName] = useState("")
  const [admin, setAdmin] = useState(false)
  const [removing, setRemoving] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      const r = await users.listUsers()
      setList(r.users)
      setMe(r.me)
    } catch (e) {
      setError(message(e))
    }
  }, [])

  useEffect(() => {
    void refresh()
    const timer = setInterval(() => void refresh(), REFRESH_MS)
    return () => clearInterval(timer)
  }, [refresh])

  async function terminal(build: () => string) {
    setError(null)
    try {
      await users.inTerminal(build())
    } catch (e) {
      setError(message(e))
    }
  }

  const row = (u: User, i: number) => (
    <Row key={u.name} first={i === 0}>
      <UserRound size={20} strokeWidth={1.6} />
      <div className="min-w-0 flex-1">
        <div className="truncate text-[14px] font-medium">
          {u.fullName || u.name}
          {u.name === me && <span className="ml-2 text-[12px] text-dim">You</span>}
        </div>
        <div className="truncate text-[12px] text-dim">
          {u.name} &nbsp;·&nbsp; {u.groups.join(", ")}
        </div>
      </div>
      {removing === u.name ? (
        <>
          <span className="text-[13px]">Delete the account and its files?</span>
          <Button onClick={() => setRemoving(null)}>Cancel</Button>
          <Button
            primary
            onClick={() => {
              setRemoving(null)
              void terminal(() => users.removeCommand(u.name))
            }}
          >
            Delete
          </Button>
        </>
      ) : (
        <>
          <span className="text-[12px] text-dim">Admin</span>
          <Switch
            on={u.admin}
            label={`${u.name} is an administrator`}
            disabled={u.name === me}
            onChange={(on) => void terminal(() => users.adminCommand(u.name, on))}
          />
          <Button onClick={() => void terminal(() => users.passwordCommand(u.name))}>Password</Button>
          {u.name !== me && <Button onClick={() => setRemoving(u.name)}>Delete</Button>}
        </>
      )}
    </Row>
  )

  const valid = users.validName(name) && !list.some((u) => u.name === name)

  return (
    <div className="max-w-[560px]">
      {error && <ErrorBanner>{error}</ErrorBanner>}
      <Group title="Accounts">
        <Card>{list.map(row)}</Card>
      </Group>
      {adding ? (
        <Group title="New user">
          <Card>
            <Row first>
              <span className="w-[100px] text-[14px]">Name</span>
              <input
                autoFocus
                aria-label="Full name"
                placeholder="Full name"
                value={fullName}
                onChange={(e) => setFullName(e.target.value)}
                className={`${inputClass} flex-1`}
              />
            </Row>
            <Row>
              <span className="w-[100px] text-[14px]">User name</span>
              <input
                aria-label="User name"
                placeholder="lowercase letters and digits"
                value={name}
                onChange={(e) => setName(e.target.value)}
                className={`${inputClass} flex-1`}
              />
            </Row>
            <Row>
              <span className="flex-1 text-[14px]">Administrator</span>
              <Switch on={admin} label="Administrator" onChange={setAdmin} />
            </Row>
          </Card>
          <div className="mt-3 flex gap-2">
            <Button onClick={() => setAdding(false)}>Cancel</Button>
            <Button
              primary
              disabled={!valid}
              onClick={() => {
                void terminal(() => users.addCommand(name, fullName, admin))
                setAdding(false)
                setName("")
                setFullName("")
                setAdmin(false)
              }}
            >
              Add user
            </Button>
          </div>
        </Group>
      ) : (
        <div className="mt-4">
          <Button onClick={() => setAdding(true)}>Add user…</Button>
        </div>
      )}
      <div className="mt-4" />
      <Note>
        Changes open a Terminal and ask for your password with sudo. The list updates when the
        command finishes.
      </Note>
    </div>
  )
}
