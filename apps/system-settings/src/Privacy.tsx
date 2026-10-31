import { apps } from "./data.ts"
import { Card, Group, Note, Row } from "./ui.tsx"

function Pill({ label }: { label: string }) {
  return <span className="rounded-full bg-fill px-2 py-0.5 text-[11px] text-dim">{label}</span>
}

export default function Privacy() {
  return (
    <div className="max-w-[520px]">
      <Note>
        Filesystem and network access the built-in apps declare. Changing permissions is not
        available yet.
      </Note>
      <Group title="Apps">
        <Card>
          {apps.map((app, i) => (
            <Row key={app.name} first={i === 0}>
              <span
                className="flex size-8 shrink-0 items-center justify-center rounded-lg text-[12px] font-semibold text-text"
                style={{ background: app.color }}
              >
                {app.symbol}
              </span>
              <div className="min-w-0 flex-1">
                <div className="text-[14px] font-medium">{app.name}</div>
                <div className="truncate text-[12px] text-dim">{app.description}</div>
              </div>
              <div className="flex shrink-0 gap-1.5">
                {app.access.map((label) => (
                  <Pill key={label} label={label} />
                ))}
              </div>
            </Row>
          ))}
        </Card>
      </Group>
    </div>
  )
}
