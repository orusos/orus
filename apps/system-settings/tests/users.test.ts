import { describe, expect, it } from "vitest"
import { addCommand, adminCommand, parseUsers, validName } from "../src/users.ts"

const passwd = [
  "root:x:0:0:root:/root:/bin/bash",
  "daemon:x:1:1:daemon:/usr/sbin:/usr/sbin/nologin",
  "partha:x:1000:1000:Partha P,,,:/home/partha:/bin/bash",
  "kid:x:1001:1001:Kid:/home/kid:/bin/bash",
  "nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin",
  "svc:x:1002:1002::/var/svc:/usr/sbin/nologin",
].join("\n")
const group = [
  "sudo:x:27:partha",
  "audio:x:29:partha,kid",
  "partha:x:1000:",
  "kid:x:1001:",
].join("\n")

describe("users", () => {
  it("lists people with their groups and admin flag, not services", () => {
    const users = parseUsers(passwd, group)
    expect(users.map((u) => u.name)).toEqual(["partha", "kid"])
    expect(users[0]).toMatchObject({ fullName: "Partha P", admin: true })
    expect(users[0].groups).toEqual(["audio", "partha", "sudo"])
    expect(users[1]).toMatchObject({ admin: false, groups: ["audio", "kid"] })
  })

  it("accepts only login names Debian accepts", () => {
    expect(validName("anna_2")).toBe(true)
    for (const bad of ["", "Anna", "1anna", "a b", "a;rm", "a".repeat(33)]) expect(validName(bad)).toBe(false)
  })

  it("builds the add command with the standard groups and quotes the full name", () => {
    const cmd = addCommand("anna", "Anna O'Neil: x", true)
    expect(cmd).toContain("--comment 'Anna O'\\''Neil  x' anna")
    expect(cmd).toMatch(/usermod -aG .*lpadmin.*,sudo anna$/)
    expect(addCommand("anna", "", false)).not.toContain("sudo anna")
    expect(() => addCommand("a;b", "x", false)).toThrow()
  })

  it("adds and drops the sudo group", () => {
    expect(adminCommand("kid", true)).toBe("sudo usermod -aG sudo kid")
    expect(adminCommand("kid", false)).toBe("sudo gpasswd -d kid sudo")
  })
})
