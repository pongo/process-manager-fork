<template>
  <div class="sidebar">

    <!-- ── Header ── -->
    <div class="sidebar-header">
      <div class="header-left">
        <span class="sidebar-title">Processes</span>
        <span v-if="store.processes.length" class="count-pill">
          <span class="count-running">{{ runningCount }}</span>
          <span class="count-sep">/</span>
          <span>{{ store.processes.length }}</span>
        </span>
      </div>
      <div class="header-actions">
        <button class="icon-btn success" @click="startAllProcesses" title="Start All">
          <svg viewBox="0 0 10 10" width="9" height="9"><path d="M2 1.5l7 3.5-7 3.5V1.5z" fill="currentColor"/></svg>
        </button>
        <button class="icon-btn danger" @click="stopAllProcesses" title="Stop All">
          <svg viewBox="0 0 10 10" width="9" height="9"><rect x="2" y="2" width="6" height="6" rx="1" fill="currentColor"/></svg>
        </button>
      </div>
    </div>

    <!-- ── Search ── -->
    <div class="search-bar">
      <svg viewBox="0 0 14 14" width="13" height="13" fill="none" class="search-icon">
        <circle cx="6" cy="6" r="4.5" stroke="var(--text-muted)" stroke-width="1.3"/>
        <path d="M9.5 9.5l2.5 2.5" stroke="var(--text-muted)" stroke-width="1.3" stroke-linecap="round"/>
      </svg>
      <input
        v-model="search"
        type="text"
        placeholder="Filter processes…"
        class="search-input"
        @keyup.escape="search = ''"
      />
      <button v-if="search" class="search-clear" @click="search = ''" title="Clear">
        <svg viewBox="0 0 10 10" width="8" height="8"><path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/></svg>
      </button>
    </div>

    <!-- ── Process list ── -->
    <div class="list">

      <div v-if="store.processes.length === 0" class="empty-state">
        <div class="empty-icon">
          <svg viewBox="0 0 40 40" width="38" height="38" fill="none">
            <rect x="4" y="8" width="32" height="22" rx="3" stroke="var(--text-muted)" stroke-width="1.5"/>
            <path d="M14 36h12M20 30v6" stroke="var(--text-muted)" stroke-width="1.5" stroke-linecap="round"/>
            <path d="M12 18l3 3 5-5" stroke="var(--accent)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
          </svg>
        </div>
        <p class="empty-title">No processes yet</p>
        <p class="empty-hint">Click <strong>New Process</strong> below to get started.</p>
      </div>

      <div v-else-if="filteredProcesses.length === 0" class="empty-state">
        <p class="empty-title">No matches</p>
        <p class="empty-hint">Try a different search term.</p>
      </div>

      <div
        v-for="p in filteredProcesses"
        :key="p.id"
        :class="['process-row', `row-${p.status.toLowerCase()}`, { selected: store.selectedProcessId === p.id }]"
        @click="store.selectedProcessId = p.id"
      >
        <!-- Main row content -->
        <div class="row-content">
          <div class="row-indicator" :class="`ind-${p.status.toLowerCase()}`"></div>
          <div class="row-body">
            <div class="row-top">
              <span class="p-name">{{ p.name }}</span>
              <span :class="['status-pill', `pill-${p.status.toLowerCase()}`]">{{ p.status }}</span>
            </div>
            <div v-if="p.status === 'Crashed'" class="row-bottom">
              <span class="p-cmd">{{ p.command }}{{ p.args?.length ? ' ' + p.args.join(' ') : '' }}</span>
              <span v-if="p.crashCount" class="crash-badge">{{ p.crashCount }}x</span>
            </div>
            <div class="row-meta" :class="{ 'meta-hidden': !p.pid || p.status !== 'Running' }">
              <template v-if="p.pid && p.status === 'Running'">
                PID {{ p.pid }}&nbsp; · &nbsp;{{ formatUptime(store.getUptime(p)) }}
              </template>
            </div>
          </div>
        </div>

        <!-- Reserved action slots prevent layout shifts on hover. -->
        <div class="row-actions">
          <button v-if="p.status !== 'Running'" class="ra-btn start" @click.stop="startProcess(p.id)" title="Start">
            <svg viewBox="0 0 10 10" width="8" height="8"><path d="M2 1.5l7 3.5-7 3.5V1.5z" fill="currentColor"/></svg>
            Start
          </button>
          <button v-if="p.status === 'Running'" class="ra-btn stop" @click.stop="stopProcess(p.id)" title="Stop">
            <svg viewBox="0 0 10 10" width="8" height="8"><rect x="1.5" y="1.5" width="7" height="7" rx="1" fill="currentColor"/></svg>
            Stop
          </button>
          <button class="ra-btn restart" @click.stop="restartProcess(p.id)" title="Restart">
            <svg viewBox="0 0 24 24" width="9" height="9" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/>
              <path d="M3 3v5h5"/>
            </svg>
            Restart
          </button>
          <button class="ra-btn remove" @click.stop="removeProcess(p.id)" title="Remove">
            <svg viewBox="0 0 10 12" width="8" height="9" fill="none">
              <path d="M1 3h8M3 3V1.5h4V3M2 3l.5 7h5L8 3" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
            Remove
          </button>
        </div>
      </div>

    </div>

    <!-- ── Footer ── -->
    <div class="sidebar-footer">
      <button class="new-process-btn" @click="showAddModal = true">
        <svg viewBox="0 0 12 12" width="11" height="11" fill="none">
          <path d="M6 1v10M1 6h10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        New Process
      </button>
    </div>

    <!-- Add process modal -->
    <AddProcessModal
      :show="showAddModal"
      @close="showAddModal = false"
      @added="onProcessAdded"
    />

  </div>
</template>

<script setup lang="ts">
import { ref, computed } from 'vue'
import { useProcessStore } from '@/stores/processStore'
import { useDialog } from '@/composables/useDialog'
import AddProcessModal from '@/components/AddProcessModal.vue'

const store = useProcessStore()
const { openConfirm, openAlert } = useDialog()

const search = ref('')
const showAddModal = ref(false)

const runningCount = computed(() =>
  store.processes.filter((p: any) => p.status === 'Running').length
)

const filteredProcesses = computed(() => {
  const q = search.value.toLowerCase()
  const processes = search.value.trim()
    ? store.processes.filter(
      (p) => p.name.toLowerCase().includes(q) || p.command.toLowerCase().includes(q)
    )
    : [...store.processes]
  return processes.sort((a, b) =>
    a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true })
    || a.id.localeCompare(b.id)
  )
})

const onProcessAdded = (id: string) => {
  store.selectedProcessId = id
}

const startProcess = async (id: string) => {
  try { await store.startProcess(id) }
  catch (e) { await openAlert('Start Failed', String(e)) }
}
const stopProcess = async (id: string) => {
  try { await store.stopProcess(id) }
  catch (e) { await openAlert('Stop Failed', String(e)) }
}
const restartProcess = async (id: string) => {
  try { await store.restartProcess(id) }
  catch (e) { await openAlert('Restart Failed', String(e)) }
}
const removeProcess = async (id: string) => {
  const p = store.processes.find((p: any) => p.id === id)
  const ok = await openConfirm(
    'Remove Process',
    `Remove "${p?.name}"? This cannot be undone.`,
    { type: 'danger', confirmLabel: 'Remove', cancelLabel: 'Cancel' }
  )
  if (!ok) return
  try { await store.removeProcess(id) }
  catch (e) { await openAlert('Remove Failed', String(e)) }
}
const startAllProcesses = async () => {
  try { await store.startAll() }
  catch (e) { await openAlert('Start All Failed', String(e)) }
}
const stopAllProcesses = async () => {
  try { await store.stopAll() }
  catch (e) { await openAlert('Stop All Failed', String(e)) }
}

const formatUptime = (ms: number) => {
  const s = Math.floor(ms / 1000)
  if (s < 60) return `${s}s`
  if (s < 3600) return `${Math.floor(s / 60)}m ${s % 60}s`
  return `${Math.floor(s / 3600)}h ${Math.floor((s % 3600) / 60)}m`
}
</script>

<style scoped>
/* ── Shell ── */
.sidebar {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: var(--bg-surface);
  overflow: hidden;
}

/* ── Header ── */
.sidebar-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0 13px;
  height: 44px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}
.header-left { display: flex; align-items: center; gap: 8px; }
.sidebar-title {
  font-size: var(--font-size-small);
  font-weight: 800;
  text-transform: uppercase;
  letter-spacing: 0.1em;
  color: var(--text-muted);
}
.count-pill {
  display: flex;
  align-items: center;
  gap: 1px;
  font-size: var(--font-size-small);
  font-weight: 600;
  background: var(--bg-raised);
  border: 1px solid var(--border);
  border-radius: 20px;
  padding: 1px 8px;
  color: var(--text-muted);
}
.count-running { color: var(--success); }
.count-sep { color: var(--text-muted); margin: 0 1px; }
.header-actions { display: flex; gap: 5px; }
.icon-btn {
  width: 26px; height: 26px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 6px; cursor: pointer;
  transition: background 0.12s, color 0.12s, border-color 0.12s;
  background: var(--bg-raised);
  color: var(--text-secondary);
  border: 1px solid var(--border-control);
}
.icon-btn.success:hover {
  background: var(--success-bg-hover);
  color: var(--success);
  border-color: var(--success-border);
}
.icon-btn.danger:hover {
  background: var(--danger-bg-hover);
  color: var(--danger);
  border-color: var(--danger-border);
}

/* ── Search ── */
.search-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 13px;
  height: 36px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  background: var(--bg-base);
}
.search-icon { flex-shrink: 0; }
.search-input {
  flex: 1; background: transparent; border: none; outline: none;
  font-size: var(--font-size-body); color: var(--text-secondary); caret-color: var(--accent);
}
.search-input::placeholder { color: var(--text-muted); }
.search-clear {
  background: none; border: none; color: var(--text-muted); cursor: pointer;
  padding: 2px; display: flex; align-items: center; transition: color 0.12s;
}
.search-clear:hover { color: var(--text-secondary); }

/* ── List ── */
.list {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
}

/* ── Empty ── */
.empty-state {
  display: flex; flex-direction: column; align-items: center;
  text-align: center; padding: 3rem 1.5rem 2rem; gap: 8px;
}
.empty-icon { margin-bottom: 6px; opacity: 0.6; }
.empty-title { font-size: var(--font-size-body); font-weight: 600; color: var(--text-muted); }
.empty-hint { font-size: var(--font-size-small); color: var(--text-muted); line-height: 1.55; }
.empty-hint strong { color: var(--accent); font-weight: 700; }

/* ── Process row ── */
.process-row {
  position: relative;
  cursor: pointer;
  transition: background 0.08s;
  border-bottom: 1px solid var(--border);
}
.process-row:last-child { border-bottom: none; }
.process-row:hover { background: var(--bg-raised); }
.process-row.selected { background: var(--bg-selected); }

.row-content {
  display: flex;
  align-items: stretch;
  padding: 10px 12px 10px 0;
  min-height: 52px;
}
.row-indicator {
  width: 3px;
  border-radius: 0 3px 3px 0;
  flex-shrink: 0;
  margin-right: 11px;
  align-self: stretch;
  transition: background 0.2s;
}
.ind-running { background: var(--success); box-shadow: 0 0 6px #22c55e55; }
.ind-stopped { background: var(--text-muted); }
.ind-crashed { background: var(--danger); box-shadow: 0 0 6px #ef444455; }
.process-row.selected .ind-stopped { background: var(--accent); }

.row-body { flex: 1; min-width: 0; }
.row-top {
  display: flex; align-items: center; gap: 7px; margin-bottom: 3px;
}
.p-name {
  flex: 1; font-size: var(--font-size-body); font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis; line-height: 1.3;
}
.process-row:hover .p-name { color: var(--text-primary); }
.process-row.selected .p-name { color: var(--text-primary); }

.status-pill {
  font-size: var(--font-size-small); font-weight: 800;
  letter-spacing: 0.06em; text-transform: uppercase;
  padding: 1.5px 6px; border-radius: 20px; flex-shrink: 0;
  border: 1px solid transparent;
}
.pill-running { background: var(--success-bg); color: var(--success); border-color: var(--success-border); }
.pill-stopped { background: var(--bg-raised); color: var(--text-muted); border-color: var(--border-control); }
.pill-crashed { background: var(--danger-bg); color: var(--danger); border-color: var(--danger-border); }

.row-bottom { display: flex; align-items: center; gap: 6px; }
.p-cmd {
  font-size: var(--font-size-small); color: var(--text-muted);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  font-family: 'Cascadia Code', 'Consolas', 'Courier New', monospace;
  flex: 1; min-width: 0;
}
.process-row:hover .p-cmd,
.process-row.selected .p-cmd { color: var(--text-secondary); }

.crash-badge {
  font-size: var(--font-size-small); font-weight: 700; color: var(--danger);
  background: var(--danger-bg); border: 1px solid var(--danger-border);
  padding: 0 5px; border-radius: 10px; flex-shrink: 0;
}

.row-meta {
  margin-top: 4px; font-size: var(--font-size-small);
  line-height: 1.4;
  min-height: 1.4em;
  color: var(--text-muted);
  font-family: 'Cascadia Code', 'Consolas', monospace;
}
.row-meta.meta-hidden { visibility: hidden; }
.process-row.selected .row-meta { color: var(--text-secondary); }

/* ── Row actions ── */
.row-actions {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  align-items: center;
  gap: 4px;
  padding: 0 12px 9px 14px;
}
.ra-btn {
  display: flex; align-items: center; justify-content: center; gap: 4px;
  min-width: 0;
  height: 28px;
  padding: 3px 6px;
  background: var(--bg-raised);
  color: var(--text-secondary);
  border: 1px solid var(--border-control); border-radius: 5px;
  cursor: pointer; font-size: var(--font-size-small); font-weight: 600;
  white-space: nowrap;
  transition: background 0.1s, border-color 0.1s;
}
.ra-btn svg { flex-shrink: 0; }
.ra-btn:hover { background: var(--bg-hover); border-color: var(--border-control-hover); }
.ra-btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.ra-btn.stop    { background: var(--danger-bg); color: var(--danger); border-color: var(--danger-border); }
.ra-btn.stop:hover { background: var(--danger-bg-hover); }

.ra-btn.remove,
.process-row:not(.row-running) .ra-btn.restart {
  visibility: hidden;
}
.process-row:hover .ra-btn,
.process-row:focus-within .ra-btn {
  visibility: visible;
}

@media (hover: none) {
  .ra-btn.remove,
  .process-row:not(.row-running) .ra-btn.restart {
    visibility: visible;
  }
}

/* ── Footer ── */
.sidebar-footer {
  padding: 10px 12px;
  border-top: 1px solid var(--border);
  flex-shrink: 0;
  background: var(--bg-base);
}
.new-process-btn {
  width: 100%;
  display: flex; align-items: center; justify-content: center; gap: 8px;
  padding: 9px;
  background: var(--accent-bg);
  border: 1.5px dashed var(--accent-border);
  border-radius: 8px;
  color: var(--accent);
  font-size: var(--font-size-body); font-weight: 700;
  cursor: pointer;
  transition: background 0.15s, border-color 0.15s, color 0.15s;
  letter-spacing: 0.01em;
}
.new-process-btn:hover {
  background: var(--accent-bg);
  border-color: var(--accent);
  border-style: solid;
  color: var(--accent);
}
</style>
