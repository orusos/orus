/** The part of the runtime's `window.host` that System Settings uses. */
interface ShellResult {
  code: number | null
  success: boolean
  stdout: string
  stderr: string
}

interface Window {
  host?: {
    shell?: {
      exec(options: { command: string; args: string[]; timeoutMs?: number }): Promise<ShellResult>
    }
  }
}
