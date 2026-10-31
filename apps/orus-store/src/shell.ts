/** Run a command through the runtime's `host.shell` capability. */

export class HostError extends Error {}

const TIMEOUT_MS = 30_000

/** Run `command` and return its whole result, whatever the exit code. */
export async function exec(
  command: string,
  args: string[],
  timeoutMs = TIMEOUT_MS,
): Promise<ShellResult> {
  const shell = window.host?.shell
  if (!shell) throw new HostError("This needs Orus OS: the shell capability is missing.")
  try {
    return await shell.exec({ command, args, timeoutMs })
  } catch (e) {
    throw new HostError(`Could not run ${command}: ${String(e)}`)
  }
}

/** Stdout of `command`; throws `HostError` with the command's stderr if it fails. */
export async function run(command: string, args: string[], timeoutMs = TIMEOUT_MS): Promise<string> {
  const result = await exec(command, args, timeoutMs)
  if (!result.success) {
    throw new HostError(result.stderr.trim() || `${command} failed (exit ${result.code})`)
  }
  return result.stdout
}

export function message(e: unknown): string {
  return e instanceof Error ? e.message : String(e)
}
