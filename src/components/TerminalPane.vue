<template>
  <div class="terminal-pane">
    <!-- Header bar -->
    <div class="terminal-header">
      <div class="terminal-header-left">
        <svg class="terminal-icon" viewBox="0 0 16 16" fill="none">
          <polyline points="2,5 7,8 2,11" stroke="var(--accent)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          <line x1="8" y1="11" x2="14" y2="11" stroke="var(--accent)" stroke-width="1.5" stroke-linecap="round"/>
        </svg>
        <span class="terminal-title">Terminal</span>
        <span class="terminal-cwd">{{ cwd }}</span>
      </div>
      <div class="terminal-header-right">
        <button class="th-btn" @click="clearOutput" title="Clear output">
          <svg viewBox="0 0 14 14" width="12" height="12">
            <path d="M2 3h10M5 3V2h4v1M3 3l.5 9h7l.5-9" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
          </svg>
        </button>
        <button class="th-btn th-close" @click="$emit('close')" title="Close terminal">
          <svg viewBox="0 0 10 10" width="10" height="10">
            <path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Output area -->
    <div class="terminal-output" ref="outputEl" @click="focusInput">
      <template v-if="lines.length === 0">
        <div class="terminal-empty">
          Type a command and press <kbd>Enter</kbd>, or <kbd>Ctrl+C</kbd> to cancel a running command.
        </div>
      </template>
      <template v-for="(block, bi) in blocks" :key="bi">
        <!-- Command line header -->
        <div class="terminal-cmd-line">
          <span class="cmd-prompt">❯</span>
          <span class="cmd-text">{{ block.command }}</span>
          <span v-if="block.running" class="cmd-badge running">running</span>
          <span v-else-if="block.exitCode === 0" class="cmd-badge ok">done</span>
          <span v-else-if="block.exitCode !== null" class="cmd-badge err">exit {{ block.exitCode }}</span>
          <div class="cmd-actions" v-if="block.running || block.exitCode !== null">
            <button v-if="block.running" class="cmd-btn kill-btn" @click.stop="killJob(block.jobId)" title="Kill process">
              <svg viewBox="0 0 10 10" width="9" height="9"><rect x="1" y="1" width="8" height="8" rx="1" fill="currentColor"/></svg>
              Stop
            </button>
            <button class="cmd-btn promote-btn" @click.stop="openPromote(block)" title="Add to Process Manager">
              <svg viewBox="0 0 10 10" width="9" height="9"><path d="M5 1v8M1 5h8" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/></svg>
              Add to processes
            </button>
          </div>
        </div>
        <!-- Output lines for this block -->
        <div
          v-for="(line, li) in block.lines"
          :key="li"
          class="terminal-line"
          :class="{ 'is-error': line.isError }"
          v-html="parseAnsi(line.text)"
        />
      </template>
    </div>

    <!-- Input bar -->
    <div class="terminal-input-bar">
      <span class="input-prompt">❯</span>
      <input
        ref="inputEl"
        v-model="inputValue"
        class="terminal-input"
        :disabled="isRunning"
        :placeholder="isRunning ? '' : 'Enter a command…'"
        spellcheck="false"
        autocomplete="off"
        autocorrect="off"
        @keydown.enter="runCommand"
        @keydown.ctrl.c.prevent="killActiveJob"
        @keydown.up.prevent="historyUp"
        @keydown.down.prevent="historyDown"
      />
      <div class="input-hint" v-if="isRunning">
        <span class="running-dot" />
        <span class="running-text">running</span>
        <button class="kill-inline" @click="killActiveJob" title="Ctrl+C">&#x25A0; stop</button>
      </div>
      <button v-else class="run-btn" @click="runCommand" :disabled="!inputValue.trim()">Run</button>
    </div>

    <!-- Promote modal -->
    <div v-if="promoteTarget" class="promote-overlay" @click.self="promoteTarget = null">
      <div class="promote-modal">
        <h3 class="promote-title">Add to Process Manager</h3>
        <p class="promote-sub">Give this process a name to track it in the sidebar.</p>
        <div class="promote-cmd-preview">{{ promoteTarget.command }}</div>
        <input
          ref="promoteInputEl"
          v-model="promoteName"
          class="promote-name-input"
          placeholder="Process name…"
          @keydown.enter="confirmPromote"
          @keydown.escape="promoteTarget = null"
        />
        <div v-if="promoteError" class="promote-error">{{ promoteError }}</div>
        <div class="promote-actions">
          <button class="promote-cancel" @click="promoteTarget = null">Cancel</button>
          <button class="promote-confirm" @click="confirmPromote" :disabled="!promoteName.trim() || promoteLoading">
            <span v-if="promoteLoading" class="spinner" />
            {{ promoteLoading ? 'Adding…' : 'Add Process' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, nextTick, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useProcessStore } from '@/stores/processStore'

const SESSION_ID = 'default'

interface OutputLine { text: string; isError: boolean }
interface Block {
  jobId: string
  command: string
  lines: OutputLine[]
  running: boolean
  exitCode: number | null
}

const store = useProcessStore()
const cwd = ref('…')
const inputValue = ref('')
const inputEl = ref<HTMLInputElement | null>(null)
const outputEl = ref<HTMLDivElement | null>(null)
const promoteInputEl = ref<HTMLInputElement | null>(null)
const blocks = ref<Block[]>([])
const lines = computed(() => blocks.value.flatMap(b => b.lines))
const history = ref<string[]>([])
const historyIdx = ref(-1)
const activeJobId = ref<string | null>(null)
const isRunning = computed(() => activeJobId.value !== null)
const promoteTarget = ref<Block | null>(null)
const promoteName = ref('')
const promoteError = ref('')
const promoteLoading = ref(false)

let unlistenOutput: UnlistenFn | null = null
let unlistenDone: UnlistenFn | null = null

onMounted(async () => {
  try { cwd.value = await invoke<string>('terminal_get_cwd', { sessionId: SESSION_ID }) } catch { /* ignore */ }
  focusInput()
  setupListeners()
})

onUnmounted(() => {
  unlistenOutput?.()
  unlistenDone?.()
})

async function setupListeners() {
  unlistenOutput = await listen<{ jobId: string; line: string; isError: boolean; timestamp: string }>(
    `terminal:output:${SESSION_ID}`,
    (e) => {
      const block = blocks.value.find(b => b.jobId === e.payload.jobId)
      if (block) {
        block.lines.push({ text: e.payload.line, isError: e.payload.isError })
        scrollToBottom()
      }
    }
  )
  unlistenDone = await listen<{ jobId: string; exitCode: number }>(
    `terminal:done:${SESSION_ID}`,
    (e) => {
      const block = blocks.value.find(b => b.jobId === e.payload.jobId)
      if (block) {
        block.running = false
        block.exitCode = e.payload.exitCode
      }
      if (activeJobId.value === e.payload.jobId) {
        activeJobId.value = null
      }
      scrollToBottom()
      focusInput()
    }
  )
}

async function runCommand() {
  const raw = inputValue.value.trim()
  if (!raw || isRunning.value) return

  // save history
  history.value.unshift(raw)
  historyIdx.value = -1
  inputValue.value = ''

  // Handle cd specially
  if (/^cd(\s|$)/i.test(raw)) {
    const path = raw.slice(2).trim()
    const block: Block = {
      jobId: `cd-${Date.now()}`,
      command: raw,
      lines: [],
      running: false,
      exitCode: null,
    }
    blocks.value.push(block)
    if (!path) {
      // just show current directory
      block.exitCode = 0
    } else {
      try {
        cwd.value = await invoke<string>('terminal_set_cwd', { sessionId: SESSION_ID, path })
        block.exitCode = 0
      } catch (e: any) {
        block.lines.push({ text: String(e), isError: true })
        block.exitCode = 1
      }
    }
    scrollToBottom()
    focusInput()
    return
  }

  const jobId = `job-${Date.now()}`
  activeJobId.value = jobId

  const block: Block = {
    jobId,
    command: raw,
    lines: [],
    running: true,
    exitCode: null,
  }
  blocks.value.push(block)
  scrollToBottom()

  try {
    await invoke('terminal_run', { sessionId: SESSION_ID, command: raw, jobId })
  } catch (e: any) {
    block.lines.push({ text: String(e), isError: true })
    block.running = false
    block.exitCode = -1
    activeJobId.value = null
    focusInput()
  }
}

async function killJob(jobId: string) {
  try { await invoke('terminal_kill', { jobId }) } catch { /* ignore */ }
}

async function killActiveJob() {
  if (activeJobId.value) await killJob(activeJobId.value)
}

function clearOutput() {
  blocks.value = []
}

function focusInput() {
  nextTick(() => inputEl.value?.focus())
}

function scrollToBottom() {
  nextTick(() => {
    if (outputEl.value) outputEl.value.scrollTop = outputEl.value.scrollHeight
  })
}

function historyUp() {
  if (history.value.length === 0) return
  historyIdx.value = Math.min(historyIdx.value + 1, history.value.length - 1)
  inputValue.value = history.value[historyIdx.value]
}

function historyDown() {
  if (historyIdx.value <= 0) { historyIdx.value = -1; inputValue.value = ''; return }
  historyIdx.value--
  inputValue.value = history.value[historyIdx.value]
}

function openPromote(block: Block) {
  promoteTarget.value = block
  promoteName.value = block.command.split(' ')[0] || ''
  promoteError.value = ''
  nextTick(() => {
    promoteInputEl.value?.focus()
    promoteInputEl.value?.select()
  })
}

async function confirmPromote() {
  if (!promoteTarget.value || !promoteName.value.trim()) return
  promoteLoading.value = true
  promoteError.value = ''
  try {
    const id = await invoke<string>('terminal_add_process', {
      name: promoteName.value.trim(),
      command: promoteTarget.value.command,
      workingDir: cwd.value || null,
    })
    await store.loadProcesses()
    store.selectedProcessId = id
    promoteTarget.value = null
    promoteName.value = ''
  } catch (e: any) {
    promoteError.value = String(e)
  } finally {
    promoteLoading.value = false
  }
}

// ── ANSI parser (same approach as ProcessPanel) ──
function parseAnsi(raw: string): string {
  const ESC = '\x1b['
  let out = ''
  let i = 0
  let openSpan = false
  const htmlEsc = (s: string) =>
    s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')
  const colorMap: Record<number, string> = {
    30: '#a3a3a3', 31: '#e06c75', 32: '#98c379', 33: '#e5c07b',
    34: '#61afef', 35: '#c678dd', 36: '#56b6c2', 37: '#abb2bf',
    90: '#b5b5b5', 91: '#e06c75', 92: '#98c379', 93: '#e5c07b',
    94: '#61afef', 95: '#c678dd', 96: '#56b6c2', 97: '#ffffff',
    40: '#242424', 41: '#450a0a', 42: '#052e16', 43: '#422006',
    44: '#172554', 45: '#2e1065', 46: '#164e63', 47: '#262626',
  }
  while (i < raw.length) {
    if (raw.startsWith(ESC, i)) {
      const end = raw.indexOf('m', i)
      if (end === -1) { i++; continue }
      const codes = raw.slice(i + 2, end).split(';').map(Number)
      if (openSpan) { out += '</span>'; openSpan = false }
      let fg = '', bg = '', bold = false
      for (const c of codes) {
        if (c === 0) { /* reset */ }
        else if (c === 1) bold = true
        else if ((c >= 30 && c <= 37) || (c >= 90 && c <= 97)) fg = colorMap[c] || ''
        else if ((c >= 40 && c <= 47)) bg = colorMap[c] || ''
      }
      if (fg || bg || bold) {
        const style = [
          fg ? `color:${fg}` : '',
          bg ? `background:${bg}` : '',
          bold ? 'font-weight:700' : '',
        ].filter(Boolean).join(';')
        out += `<span style="${style}">`
        openSpan = true
      }
      i = end + 1
    } else {
      out += htmlEsc(raw[i])
      i++
    }
  }
  if (openSpan) out += '</span>'
  return out
}
</script>

<style scoped>
.terminal-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-base);
  font-family: 'Cascadia Code', 'Fira Code', 'Consolas', monospace;
  font-size: var(--font-size-body);
  overflow: hidden;
  position: relative;
}

/* ── Header ── */
.terminal-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 34px;
  padding: 0 10px 0 12px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  user-select: none;
}
.terminal-header-left { display: flex; align-items: center; gap: 8px; overflow: hidden; min-width: 0; }
.terminal-header-right { display: flex; align-items: center; gap: 4px; flex-shrink: 0; }
.terminal-icon { width: 14px; height: 14px; flex-shrink: 0; }
.terminal-title { font-size: var(--font-size-body); font-weight: 600; color: var(--text-secondary); letter-spacing: 0.02em; flex-shrink: 0; }
.terminal-cwd {
  font-size: var(--font-size-small);
  color: var(--text-muted);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  direction: rtl;
  text-align: left;
  max-width: 400px;
}

.th-btn {
  width: 26px; height: 26px;
  display: flex; align-items: center; justify-content: center;
  background: transparent; border: none;
  color: var(--text-muted); cursor: pointer; border-radius: 4px;
  transition: background 0.12s, color 0.12s;
}
.th-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
.th-close:hover { background: var(--danger-bg-hover) !important; color: var(--danger-hover) !important; }

/* ── Output ── */
.terminal-output {
  flex: 1;
  overflow-y: auto;
  padding: 8px 12px;
  display: flex;
  flex-direction: column;
  gap: 1px;
  cursor: text;
}

.terminal-empty {
  color: var(--text-muted);
  font-family: inherit;
  font-size: var(--font-size-small);
  padding: 12px 0;
  user-select: none;
}
.terminal-empty kbd {
  background: var(--bg-hover);
  border: 1px solid var(--border-control);
  border-radius: 3px;
  padding: 1px 5px;
  color: var(--accent);
  font-family: inherit;
}

/* Command block header */
.terminal-cmd-line {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 10px;
  flex-wrap: wrap;
}
.terminal-cmd-line:first-child { margin-top: 0; }
.cmd-prompt { color: var(--accent); font-weight: 700; flex-shrink: 0; }
.cmd-text { color: var(--text-primary); word-break: break-all; flex: 1; min-width: 0; }
.cmd-badge {
  font-size: var(--font-size-small);
  font-weight: 600;
  letter-spacing: 0.05em;
  padding: 1px 5px;
  border-radius: 3px;
  flex-shrink: 0;
}
.cmd-badge.running { background: var(--info-bg); color: var(--info); }
.cmd-badge.ok { background: var(--success-bg); color: var(--success); }
.cmd-badge.err { background: var(--danger-bg); color: var(--danger); }
.cmd-actions { display: flex; align-items: center; gap: 4px; margin-left: auto; }
.cmd-btn {
  display: flex; align-items: center; gap: 3px;
  padding: 2px 7px;
  background: var(--bg-hover); border: 1px solid var(--border-control);
  color: var(--text-secondary); cursor: pointer; border-radius: 4px;
  font-size: var(--font-size-small); font-family: inherit;
  transition: background 0.12s, border-color 0.12s;
}
.kill-btn:hover { background: var(--danger-bg); border-color: var(--danger-border); color: var(--danger); }
.promote-btn:hover { background: var(--success-bg); border-color: var(--success-border); color: var(--success); }

/* Output lines */
.terminal-line {
  color: var(--text-secondary);
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-all;
  padding: 0 0 0 18px;
}
.terminal-line.is-error { color: var(--danger); }

/* ── Input bar ── */
.terminal-input-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 10px 8px;
  background: var(--bg-surface);
  border-top: 1px solid var(--border);
  flex-shrink: 0;
}
.input-prompt { color: var(--accent); font-weight: 700; font-size: var(--font-size-body); }
.terminal-input {
  flex: 1;
  min-width: 0;
  background: transparent;
  border: none;
  outline: none;
  color: var(--text-primary);
  font-family: inherit;
  font-size: inherit;
  caret-color: var(--accent);
}
.terminal-input::placeholder { color: var(--text-muted); }
.terminal-input:disabled { opacity: 0.5; cursor: not-allowed; }

.input-hint {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.running-dot {
  width: 6px; height: 6px;
  border-radius: 50%;
  background: var(--info);
  animation: pulse 1s infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}
.running-text { font-size: var(--font-size-small); color: var(--text-muted); }
.kill-inline {
  background: transparent; border: 1px solid var(--border-control);
  color: var(--text-muted); cursor: pointer; border-radius: 3px;
  font-size: var(--font-size-small); padding: 1px 6px;
  font-family: inherit;
  transition: all 0.12s;
}
.kill-inline:hover { border-color: var(--danger-border); color: var(--danger); background: var(--danger-bg); }

.run-btn {
  padding: 3px 10px;
  background: var(--accent-bg-hover); border: 1px solid var(--accent);
  color: var(--accent); cursor: pointer; border-radius: 4px;
  font-size: var(--font-size-small); font-family: inherit;
  font-weight: 600; flex-shrink: 0;
  transition: background 0.12s, border-color 0.12s;
}
.run-btn:hover:not(:disabled) { background: var(--accent-bg-hover); border-color: var(--accent); color: var(--accent-hover); }
.run-btn:disabled { opacity: 0.3; cursor: not-allowed; }

/* ── Promote modal ── */
.promote-overlay {
  position: absolute;
  inset: 0;
  background: rgba(0,0,0,0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
  backdrop-filter: blur(3px);
  padding: 12px;
}
.promote-modal {
  background: var(--bg-surface);
  border: 1px solid var(--border-control);
  border-radius: 10px;
  padding: 20px 22px;
  width: 360px;
  max-width: 100%;
  max-height: 100%;
  overflow-y: auto;
  box-shadow: 0 20px 40px rgba(0,0,0,0.6);
}
.promote-title { font-size: var(--font-size-title); font-weight: 600; color: var(--text-primary); margin-bottom: 4px; }
.promote-sub { font-size: var(--font-size-small); color: var(--text-muted); margin-bottom: 12px; }
.promote-cmd-preview {
  background: var(--bg-base);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 6px 10px;
  font-size: var(--font-size-body);
  color: var(--accent);
  margin-bottom: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.promote-name-input {
  width: 100%;
  background: var(--bg-base);
  border: 1px solid var(--border-control);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: var(--font-size-body);
  padding: 7px 10px;
  outline: none;
  transition: border-color 0.15s;
  box-sizing: border-box;
}
.promote-name-input:focus { border-color: var(--accent); }
.promote-error { color: var(--danger); font-size: var(--font-size-small); margin-top: 6px; }
.promote-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}
.promote-cancel {
  padding: 6px 14px;
  background: transparent; border: 1px solid var(--border-control);
  color: var(--text-muted); border-radius: 6px; cursor: pointer;
  font-size: var(--font-size-body);
  transition: all 0.12s;
}
.promote-cancel:hover { background: var(--bg-hover); color: var(--text-secondary); }
.promote-confirm {
  padding: 6px 14px;
  background: var(--success-bg); border: 1px solid var(--success-border);
  color: var(--success); border-radius: 6px; cursor: pointer;
  font-size: var(--font-size-body); font-weight: 600;
  display: flex; align-items: center; gap: 6px;
  transition: all 0.12s;
}
.promote-confirm:hover:not(:disabled) { background: var(--success-bg-hover); border-color: var(--success-border); }
.promote-confirm:disabled { opacity: 0.5; cursor: not-allowed; }

.spinner {
  width: 10px; height: 10px;
  border: 2px solid rgba(74,222,128,0.3);
  border-top-color: #4ade80;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  display: inline-block;
}
@keyframes spin { to { transform: rotate(360deg); } }

/* Scrollbar */
.terminal-output::-webkit-scrollbar { width: 4px; }
.terminal-output::-webkit-scrollbar-track { background: transparent; }
.terminal-output::-webkit-scrollbar-thumb { background: var(--scrollbar); border-radius: 2px; }
</style>
