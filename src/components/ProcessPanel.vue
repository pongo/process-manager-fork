<template>
  <div class="panel">
    <!-- Empty state -->
    <div v-if="!proc" class="empty-state">
      <svg viewBox="0 0 48 48" width="48" height="48" fill="none">
        <rect x="4" y="8" width="40" height="28" rx="3" stroke="var(--text-muted)" stroke-width="2"/>
        <path d="M16 44h16M24 36v8" stroke="var(--text-muted)" stroke-width="2" stroke-linecap="round"/>
        <path d="M14 20l4 4-4 4M22 28h8" stroke="var(--text-muted)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
      </svg>
      <p>Select a process</p>
      <span>Choose one from the sidebar to inspect it</span>
    </div>

    <!-- Process detail -->
    <div v-else class="panel-body">

      <!-- ── Header ── -->
      <div class="panel-header">
        <div class="hdr-left">
          <span :class="['status-dot', `dot-${proc.status.toLowerCase()}`]"></span>
          <h2 class="proc-title">{{ proc.name }}</h2>
          <span :class="['status-badge', `sb-${proc.status.toLowerCase()}`]">{{ proc.status }}</span>
        </div>
        <div class="hdr-right">
          <button v-if="proc.status !== 'Running'" class="hdr-btn start" @click="startProcess">
            <svg viewBox="0 0 10 10" width="9" height="9"><path d="M2 1.5l7 3.5-7 3.5V1.5z" fill="currentColor"/></svg>
            Start
          </button>
          <button v-if="proc.status === 'Running'" class="hdr-btn stop" @click="stopProcess">
            <svg viewBox="0 0 10 10" width="9" height="9"><rect x="2" y="2" width="6" height="6" rx="1" fill="currentColor"/></svg>
            Stop
          </button>
          <button class="hdr-btn restart" @click="restartProcess">
            <svg viewBox="0 0 24 24" width="10" height="10" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
              <path d="M3 3v5h5"/>
            </svg>
            Restart
          </button>
          <button :class="['hdr-btn', 'settings-btn', showSettings ? 'active' : '']" @click="showSettings = !showSettings">
            Settings
          </button>
          <button class="hdr-btn close-btn" @click="closePanel" title="Close">
            Close
          </button>
        </div>
      </div>

      <!-- ── Metrics ── -->
      <div class="metrics-row">
        <div class="metric">
          <span class="m-label">CPU</span>
          <span class="m-val" :class="{ 'val-dim': proc.status !== 'Running' }">
            {{ proc.status === 'Running' ? metrics.cpuPercent.toFixed(1) + '%' : '—' }}
          </span>
        </div>
        <div class="metric">
          <span class="m-label">Memory</span>
          <span class="m-val" :class="{ 'val-dim': proc.status !== 'Running' }">
            <span v-if="proc.status === 'Running'">
              {{ metrics.memoryMb }} MB
              <span class="m-sub">&thinsp;{{ metrics.memoryPercent.toFixed(1) }}%</span>
            </span>
            <span v-else>—</span>
          </span>
        </div>
        <div class="metric">
          <span class="m-label">PID</span>
          <span class="m-val" :class="{ 'val-dim': !proc.pid }">{{ proc.pid ?? '—' }}</span>
        </div>
        <div class="metric">
          <span class="m-label">Uptime</span>
          <span class="m-val" :class="{ 'val-dim': !store.getUptime(proc) }">{{ proc.status === 'Running' || proc.uptimeMs ? formatUptime(store.getUptime(proc)) : '—' }}</span>
        </div>
        <div class="metric">
          <span class="m-label">Crashes</span>
          <span class="m-val" :class="proc.crashCount ? 'val-crash' : 'val-dim'">{{ proc.crashCount }}</span>
        </div>
        <div class="metric">
          <span class="m-label">Command</span>
          <span class="m-val m-mono">{{ proc.command }}{{ proc.args?.length ? ' ' + proc.args.join(' ') : '' }}</span>
        </div>
      </div>

      <!-- ── Settings panel ── -->
      <transition name="settings-slide">
        <div v-if="showSettings" class="settings-panel">
          <div class="settings-grid">
            <div class="settings-col">
              <div class="settings-section-title">Behavior</div>
              <label class="toggle-row">
                <div class="toggle-info">
                  <span class="toggle-label">Auto-restart on crash</span>
                  <span class="toggle-hint">Restart automatically if the process exits unexpectedly</span>
                </div>
                <div :class="['toggle', { on: draft.autoRestart }]" @click="draft.autoRestart = !draft.autoRestart">
                  <div class="toggle-thumb"></div>
                </div>
              </label>
              <label class="toggle-row">
                <div class="toggle-info">
                  <span class="toggle-label">Auto-start on app launch</span>
                  <span class="toggle-hint">Start this process when Process Manager opens</span>
                </div>
                <div :class="['toggle', { on: draft.autoStart }]" @click="draft.autoStart = !draft.autoStart">
                  <div class="toggle-thumb"></div>
                </div>
              </label>
            </div>
            <div class="settings-col">
              <div class="settings-section-title">Process</div>
              <div class="settings-field">
                <label class="sf-label">Working Directory</label>
                <input v-model="draft.workingDir" type="text" class="sf-input"
                  placeholder="Default: inherited from app" />
              </div>
            </div>
          </div>
          <div class="settings-footer">
            <button class="settings-save" @click="saveSettings" :disabled="saving">
              {{ saving ? 'Saving...' : 'Save Settings' }}
            </button>
            <span v-if="savedMsg" class="saved-msg">✓ Saved</span>
          </div>
        </div>
      </transition>

      <!-- ── Log viewer ── -->
      <div class="logs-section">
        <div class="logs-toolbar">
          <span class="logs-label">OUTPUT</span>
          <div class="logs-actions">
            <label class="autoscroll" @click="autoScroll = !autoScroll">
              <span class="as-track" :class="{ on: autoScroll }">
                <span class="as-thumb"></span>
              </span>
              <span class="as-text">Auto-scroll</span>
            </label>
            <button class="log-btn" @click="refreshLogs">Refresh</button>
            <button class="log-btn danger" @click="clearLogs">Clear</button>
          </div>
        </div>
        <div class="logs-body" ref="logsEl">
          <div
            v-for="(log, i) in currentLogs"
            :key="i"
            :class="['log-line', `ll-${log.level}`]"
          >
            <span class="ll-ts">{{ formatTs(log.timestamp) }}</span>
            <span :class="['ll-lvl', `lvl-${log.level}`]">{{ log.level }}</span>
            <!-- eslint-disable-next-line vue/no-v-html -->
            <span class="ll-msg" v-html="parseAnsi(log.message)"></span>
          </div>
          <div v-if="currentLogs.length === 0" class="no-output">No output yet…</div>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch, nextTick, onBeforeUnmount } from 'vue'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'
import { useProcessStore } from '@/stores/processStore'
import { useDialog } from '@/composables/useDialog'
import { windowActive } from '@/composables/useWindowActivity'
import { usePolling } from '@/composables/usePolling'

const store = useProcessStore()
const { openConfirm, openAlert } = useDialog()
const showSettings = ref(false)
const autoScroll = ref(true)
const saving = ref(false)
const savedMsg = ref(false)
const logsEl = ref<HTMLElement>()
const metrics = reactive({ cpuPercent: 0, memoryMb: 0, memoryPercent: 0 })
const draft = reactive({ autoRestart: false, autoStart: false, workingDir: '' })

const proc = computed(() => store.selectedProcess())
const currentLogs = computed(() =>
  store.selectedProcessId ? store.getProcessLogs(store.selectedProcessId) : []
)

let unlistenLog: UnlistenFn | null = null

const scrollToBottom = async () => {
  if (!autoScroll.value) return
  await nextTick()
  if (logsEl.value) logsEl.value.scrollTop = logsEl.value.scrollHeight
}

const syncDraft = () => {
  const p = proc.value
  if (!p) return
  draft.autoRestart = p.autoRestart
  draft.autoStart = p.autoStart
  draft.workingDir = p.workingDir ?? ''
}

const metricsTarget = computed(() => {
  const p = proc.value
  return windowActive.value && p?.status === 'Running' && p.pid ? `${p.id}:${p.pid}` : ''
})
let metricsRevision = 0
watch(metricsTarget, () => {
  metricsRevision++
  Object.assign(metrics, { cpuPercent: 0, memoryMb: 0, memoryPercent: 0 })
}, { flush: 'sync' })

usePolling(async () => {
  const target = metricsTarget.value
  const revision = metricsRevision
  const id = store.selectedProcessId
  if (!target || !id) return
  const isCurrent = () => metricsRevision === revision && metricsTarget.value === target
  await store.updateMetrics(id, isCurrent)
  if (!isCurrent()) return
  Object.assign(metrics, store.getProcessMetrics(id))
}, metricsTarget)

const stopLogListener = () => {
  if (unlistenLog) { unlistenLog(); unlistenLog = null }
}

onBeforeUnmount(() => {
  metricsRevision++
  stopLogListener()
})

watch(() => store.selectedProcessId, async (id) => {
  metrics.cpuPercent = 0
  metrics.memoryMb = 0
  metrics.memoryPercent = 0
  showSettings.value = false
  stopLogListener()
  if (!id) return
  syncDraft()
  await loadLogs()
  unlistenLog = await listen(`process:log:${id}`, (event) => {
    const log = event.payload as { timestamp: string; level: string; message: string }
    store.addLog(id, {
      timestamp: log.timestamp,
      level: (log.level === 'stdout' ? 'stdout' : 'stderr') as 'stdout' | 'stderr',
      message: log.message,
    })
  })
  if (store.selectedProcessId !== id) { stopLogListener() }
})

watch(showSettings, (val) => { if (val) syncDraft() })

// Auto-scroll on new logs
watch(() => currentLogs.value.length, () => scrollToBottom())

const loadLogs = async () => {
  if (!store.selectedProcessId) return
  try { await store.loadLogs(store.selectedProcessId) }
  catch (e) { console.error('Failed to load logs:', e) }
  await scrollToBottom()
}

const startProcess = async () => {
  if (!store.selectedProcessId) return
  try { await store.startProcess(store.selectedProcessId) }
  catch (e) { await openAlert('Start Failed', String(e)) }
}

const stopProcess = async () => {
  if (!store.selectedProcessId) return
  try { await store.stopProcess(store.selectedProcessId) }
  catch (e) { await openAlert('Stop Failed', String(e)) }
}

const restartProcess = async () => {
  if (!store.selectedProcessId) return
  try { await store.restartProcess(store.selectedProcessId) }
  catch (e) { await openAlert('Restart Failed', String(e)) }
}

const closePanel = () => {
  store.selectedProcessId = null
}

const saveSettings = async () => {
  if (!store.selectedProcessId) return
  saving.value = true
  try {
    await store.updateProcess(
      store.selectedProcessId,
      draft.autoRestart,
      draft.autoStart,
      draft.workingDir
    )
    await store.saveConfig()
    savedMsg.value = true
    setTimeout(() => { savedMsg.value = false }, 2000)
  } catch (e) {
    await openAlert('Save Failed', String(e))
  } finally {
    saving.value = false
  }
}

const refreshLogs = async () => { await loadLogs() }

const clearLogs = async () => {
  if (!store.selectedProcessId) return
  const ok = await openConfirm(
    'Clear Logs',
    'Delete all output logs for this process? This cannot be undone.',
    { type: 'danger', confirmLabel: 'Clear Logs', cancelLabel: 'Cancel' }
  )
  if (!ok) return
  try { await store.clearLogs(store.selectedProcessId) }
  catch (e) { await openAlert('Clear Failed', String(e)) }
}

const formatUptime = (ms: number) => {
  const s = Math.floor(ms / 1000)
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`
}

const formatTs = (ts: string) => ts.length > 12 ? ts.slice(11, 19) : ts

// ── ANSI colour parser ──────────────────────────────────────────────────────
// Maps standard ANSI SGR colour codes to terminal-like hex colours.
const ANSI_FG: Record<number, string> = {
  30: '#a3a3a3', 31: '#f87171', 32: '#4ade80', 33: '#facc15',
  34: '#60a5fa', 35: '#c084fc', 36: '#22d3ee', 37: '#e2e8f0',
  90: '#b5b5b5', 91: '#fb923c', 92: '#86efac', 93: '#fde047',
  94: '#93c5fd', 95: '#d8b4fe', 96: '#67e8f9', 97: '#ffffff',
}
const ANSI_BG: Record<number, string> = {
  40: '#1a1a1a', 41: '#450a0a', 42: '#052e16', 43: '#422006',
  44: '#172554', 45: '#2e1065', 46: '#164e63', 47: '#1e293b',
}

function parseAnsi(raw: string): string {
  // Escape HTML first so injected content is safe
  const escaped = raw
    .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;')

  let out = ''
  let bold = false
  let fg = ''
  let bg = ''
  let spanOpen = false

  const closeSpan = () => {
    if (spanOpen) { out += '</span>'; spanOpen = false }
  }
  const openSpan = () => {
    const styles: string[] = []
    if (fg) styles.push(`color:${fg}`)
    if (bg) styles.push(`background:${bg}`)
    if (bold) styles.push('font-weight:700')
    if (styles.length) {
      out += `<span style="${styles.join(';')}"`; out += '>'; spanOpen = true
    }
  }

  // Split on ANSI escape sequences
  const parts = escaped.split(/(\x1b\[[0-9;]*m)/)
  for (const part of parts) {
    const esc = part.match(/^\x1b\[([0-9;]*)m$/)
    if (esc) {
      closeSpan()
      const codes = esc[1] === '' ? [0] : esc[1].split(';').map(Number)
      for (const code of codes) {
        if (code === 0) { bold = false; fg = ''; bg = '' }
        else if (code === 1) bold = true
        else if (code === 22) bold = false
        else if (ANSI_FG[code]) fg = ANSI_FG[code]
        else if (ANSI_BG[code]) bg = ANSI_BG[code]
        else if (code === 39) fg = ''
        else if (code === 49) bg = ''
      }
      openSpan()
    } else if (part) {
      out += part
    }
  }
  closeSpan()
  return out || escaped
}
</script>
<style scoped>
/* ── Shell ── */
.panel {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-base);
  overflow: hidden;
}

/* ── Empty state ── */
.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  color: var(--text-muted);
}
.empty-state p { font-size: var(--font-size-title); font-weight: 600; color: var(--text-muted); }
.empty-state span { font-size: var(--font-size-small); color: var(--text-muted); }

/* ── Panel body ── */
.panel-body {
  display: flex;
  flex-direction: column;
  height: 100%;
  overflow-y: auto;
}

/* ── Header ── */
.panel-header {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  padding: 11px 16px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  gap: 10px;
}
.hdr-left { display: flex; align-items: center; gap: 9px; min-width: 0; max-width: 100%; overflow: hidden; }

.status-dot {
  width: 9px; height: 9px;
  border-radius: 50%;
  flex-shrink: 0;
}
.dot-running { background: var(--success); box-shadow: 0 0 7px #22c55eaa; }
.dot-stopped { background: var(--text-muted); }
.dot-crashed { background: var(--danger); box-shadow: 0 0 7px #ef4444aa; }

.proc-title {
  margin: 0;
  font-size: var(--font-size-title);
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.status-badge {
  font-size: var(--font-size-small);
  font-weight: 700;
  letter-spacing: 0.05em;
  text-transform: uppercase;
  padding: 2px 8px;
  border-radius: 20px;
  flex-shrink: 0;
}
.sb-running { background: var(--success-bg); color: var(--success); border: 1px solid var(--success-border); }
.sb-stopped { background: var(--bg-raised); color: var(--text-muted); border: 1px solid var(--border-control); }
.sb-crashed { background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger-border); }

.hdr-right { display: flex; align-items: center; gap: 5px; flex-shrink: 0; }

.hdr-btn {
  display: flex;
  align-items: center;
  gap: 5px;
  padding: 5px 11px;
  border: 1px solid transparent;
  border-radius: 5px;
  cursor: pointer;
  font-size: var(--font-size-body);
  font-weight: 500;
  transition: opacity 0.12s, background 0.12s;
  white-space: nowrap;
}
.hdr-btn:hover { filter: brightness(1.15); }
.hdr-btn.start   { background: var(--success-bg); color: var(--success); border-color: var(--success-border); }
.hdr-btn.stop    { background: var(--danger-bg); color: var(--danger); border-color: var(--danger-border); }
.hdr-btn.restart { background: var(--warning-bg); color: var(--warning); border-color: var(--warning-border); }
.hdr-btn.settings-btn,
.hdr-btn.close-btn { background: var(--bg-raised); color: var(--text-muted); border-color: var(--border-control); }
.hdr-btn.settings-btn.active { background: var(--bg-hover); color: var(--text-secondary); border-color: var(--border-control-hover); }
.hdr-btn.close-btn:hover { color: var(--text-primary); }

/* ── Metrics ── */
.metrics-row {
  display: flex;
  flex-wrap: nowrap;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  overflow-x: auto;
}
.metric {
  flex: 1;
  min-width: 112px;
  padding: 9px 14px;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  gap: 3px;
}
.metric:last-child { border-right: none; }
.m-label {
  font-size: var(--font-size-small);
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.07em;
  color: var(--text-muted);
}
.m-val {
  font-size: var(--font-size-body);
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.m-val.val-dim { color: var(--text-muted); font-weight: 400; }
.m-val.val-crash { color: var(--danger); }
.m-sub { font-size: var(--font-size-small); color: var(--text-muted); font-weight: 400; }
.m-mono { font-family: 'Cascadia Code', 'Courier New', monospace; font-size: var(--font-size-small) !important; }

/* ── Settings panel ── */
.settings-panel {
  border-bottom: 1px solid var(--border);
  background: var(--bg-surface);
  flex-shrink: 0;
  max-height: 40vh;
  overflow-y: auto;
}
.settings-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 0;
  padding: 14px 18px;
}
.settings-col { padding-right: 18px; min-width: 0; }
.settings-col:last-child {
  padding-right: 0;
  border-left: 1px solid var(--border);
  padding-left: 18px;
}
.settings-section-title {
  font-size: var(--font-size-small);
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.toggle-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.toggle-row:last-of-type { border-bottom: none; }
.toggle-info { flex: 1; min-width: 0; }
.toggle-label { display: block; font-size: var(--font-size-body); color: var(--text-secondary); }
.toggle-hint { display: block; font-size: var(--font-size-small); color: var(--text-muted); margin-top: 1px; }

.toggle {
  width: 32px; height: 17px;
  border-radius: 9px;
  background: var(--bg-raised);
  border: 1px solid var(--border-control);
  position: relative;
  cursor: pointer;
  flex-shrink: 0;
  transition: background 0.2s, border-color 0.2s;
}
.toggle.on { background: var(--accent-fill); border-color: var(--accent); }
.toggle-thumb {
  position: absolute;
  top: 1.5px; left: 1.5px;
  width: 12px; height: 12px;
  border-radius: 50%;
  background: var(--text-secondary);
  transition: transform 0.2s, background 0.2s;
}
.toggle.on .toggle-thumb { transform: translateX(15px); background: #fff; }

.settings-field { margin-top: 4px; }
.sf-label {
  display: block;
  font-size: var(--font-size-small);
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--text-muted);
  margin-bottom: 5px;
}
.sf-input {
  width: 100%;
  padding: 6px 9px;
  background: var(--bg-raised);
  border: 1px solid var(--border-control);
  border-radius: 5px;
  color: var(--text-secondary);
  font-size: var(--font-size-body);
  outline: none;
  transition: border-color 0.12s;
  font-family: 'Cascadia Code', 'Courier New', monospace;
}
.sf-input:focus { border-color: var(--accent); color: var(--text-primary); }
.sf-input::placeholder { color: var(--text-muted); }

.settings-footer {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 18px 12px;
}
.settings-save {
  padding: 6px 18px;
  background: var(--accent-bg);
  border: 1px solid var(--accent-border);
  border-radius: 5px;
  color: var(--accent);
  font-size: var(--font-size-body);
  font-weight: 600;
  cursor: pointer;
  transition: background 0.12s;
}
.settings-save:hover:not(:disabled) { background: var(--accent-bg-hover); }
.settings-save:disabled { opacity: 0.4; cursor: not-allowed; }
.saved-msg { font-size: var(--font-size-small); color: var(--success); }

.settings-slide-enter-active,
.settings-slide-leave-active { transition: all 0.16s ease; overflow: hidden; }
.settings-slide-enter-from,
.settings-slide-leave-to { opacity: 0; max-height: 0; }
.settings-slide-enter-to,
.settings-slide-leave-from { opacity: 1; max-height: 400px; }

/* ── Logs ── */
.logs-section {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 110px;
  overflow: hidden;
}
.logs-toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 14px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.logs-label {
  font-size: var(--font-size-small);
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.09em;
  color: var(--text-muted);
}
.logs-actions { display: flex; align-items: center; gap: 8px; }

.autoscroll { display: flex; align-items: center; gap: 6px; cursor: pointer; user-select: none; }
.as-track {
  width: 26px; height: 14px;
  border-radius: 7px;
  background: var(--bg-raised);
  border: 1px solid var(--border-control);
  position: relative;
  cursor: pointer;
  transition: background 0.2s;
}
.as-track.on { background: var(--accent-bg); border-color: var(--accent); }
.as-thumb {
  position: absolute;
  top: 1px; left: 1px;
  width: 10px; height: 10px;
  border-radius: 50%;
  background: var(--text-muted);
  transition: transform 0.2s, background 0.2s;
}
.as-track.on .as-thumb { transform: translateX(12px); background: var(--accent); }
.as-text { font-size: var(--font-size-small); color: var(--text-muted); }

.log-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 3px 9px;
  background: var(--bg-raised);
  border: 1px solid var(--border);
  border-radius: 4px;
  color: var(--text-muted);
  font-size: var(--font-size-small);
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.log-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
.log-btn.danger { border-color: var(--danger-border); color: var(--danger); background: var(--danger-bg); }
.log-btn.danger:hover { background: var(--danger-bg); color: var(--danger-hover); border-color: var(--danger-border); }

.logs-body {
  flex: 1;
  overflow-y: auto;
  padding: 6px 0;
  font-family: 'Cascadia Code', 'Fira Code', 'Courier New', monospace;
  font-size: var(--font-size-body);
  line-height: 1.6;
  min-height: 0;
}
.log-line {
  display: flex;
  padding: 0 4px;
}
.log-line:hover { background: var(--bg-raised); }
.ll-ts {
  color: var(--text-muted);
  font-size: var(--font-size-small);
  min-width: 80px;
  flex-shrink: 0;
  padding: 0 6px 0 10px;
  line-height: inherit;
}
.ll-lvl {
  font-size: var(--font-size-small);
  font-weight: 700;
  min-width: 60px;
  flex-shrink: 0;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  line-height: inherit;
  padding-right: 6px;
}
.lvl-stdout { color: var(--text-muted); }
.lvl-stderr { color: var(--danger); }
.ll-stdout .ll-msg { color: var(--text-secondary); }
.ll-stderr .ll-msg { color: var(--danger-hover); }
.ll-msg { flex: 1; min-width: 0; word-break: break-all; white-space: pre-wrap; }

.no-output {
  padding: 2.5rem;
  text-align: center;
  color: var(--text-muted);
  font-size: var(--font-size-body);
  font-family: sans-serif;
}
</style>
