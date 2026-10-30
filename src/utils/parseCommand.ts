export function parseCommand(raw: string): { cmd: string; args: string[] } {
  const parts: string[] = []
  let part = ''
  let quoted = false
  let started = false

  for (let i = 0; i < raw.length; i++) {
    const char = raw[i]
    if (char === '\\') {
      let end = i
      while (raw[end] === '\\') end++
      const count = end - i
      // Windows only treats backslashes as escapes immediately before a quote.
      if (raw[end] === '"') {
        part += '\\'.repeat(Math.floor(count / 2))
        if (count % 2) part += '"'
        else quoted = !quoted
        i = end
      } else {
        part += '\\'.repeat(count)
        i = end - 1
      }
      started = true
    } else if (char === '"') {
      quoted = !quoted
      started = true
    } else if (/\s/.test(char) && !quoted) {
      if (started) parts.push(part)
      part = ''
      started = false
    } else {
      part += char
      started = true
    }
  }

  if (quoted) throw new Error('Unclosed double quote in command.')
  if (started) parts.push(part)
  const cmd = parts.shift() ?? ''
  if (!cmd) throw new Error('Command is required.')
  return { cmd, args: parts }
}
