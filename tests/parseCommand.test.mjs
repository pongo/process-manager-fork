import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import { test } from 'node:test'
import ts from 'typescript'

const source = readFileSync(new URL('../src/utils/parseCommand.ts', import.meta.url), 'utf8')
const { outputText } = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2020 },
})
const { parseCommand } = await import(`data:text/javascript;base64,${Buffer.from(outputText).toString('base64')}`)

test('parses quoted and unquoted Windows paths', () => {
  for (const raw of [String.raw`node "C:\temp\t.js"`, String.raw`node C:\temp\t.js`]) {
    assert.deepEqual(parseCommand(raw), { cmd: 'node', args: [String.raw`C:\temp\t.js`] })
  }
  assert.deepEqual(parseCommand(String.raw`"C:\Program Files\nodejs\node.exe" "C:\my scripts\t.js"`), {
    cmd: String.raw`C:\Program Files\nodejs\node.exe`, args: [String.raw`C:\my scripts\t.js`],
  })
})

test('preserves empty arguments, embedded quoted segments and escaped quotes', () => {
  assert.deepEqual(parseCommand(String.raw` node  "" --name="hello world" "say \"hello\"" "C:\temp\\" `), {
    cmd: 'node', args: ['', '--name=hello world', 'say "hello"', 'C:\\temp\\'],
  })
})

test('rejects missing executable and unclosed quotes', () => {
  for (const raw of ['', '   ', '"" arg', 'node "unfinished']) {
    assert.throws(() => parseCommand(raw))
  }
})
