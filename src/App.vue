<template>
  <div class="app">
    <!-- Custom frameless titlebar -->
    <div class="titlebar" data-tauri-drag-region>
      <div class="titlebar-left" data-tauri-drag-region>
        <svg class="app-logo" viewBox="0 0 16 16" fill="none">
          <path d="M9.5 1L3 9h5.5L6.5 15 13 7H7.5L9.5 1z" fill="var(--accent)" stroke="var(--accent)" stroke-width="0.5" stroke-linejoin="round"/>
        </svg>
        <span class="app-name">Process Manager</span>
      </div>
      <div class="titlebar-center" data-tauri-drag-region>
        <span class="process-count" v-if="store.processes.length">
          {{ runningCount }}/{{ store.processes.length }} running
        </span>
      </div>
      <div class="titlebar-controls">
        <button class="wc wc-min" @click.stop="minimize" title="Minimize">
          <svg viewBox="0 0 10 1" width="10" height="1"><rect width="10" height="1" fill="currentColor"/></svg>
        </button>
        <button class="wc wc-max" @click.stop="toggleMaximize" title="Maximize">
          <svg viewBox="0 0 10 10" width="10" height="10"><rect x="0.5" y="0.5" width="9" height="9" stroke="currentColor" fill="none"/></svg>
        </button>
        <button class="wc wc-close" @click.stop="hideToTray" title="Minimize to tray">
          <svg viewBox="0 0 10 10" width="10" height="10">
            <path d="M1 1l8 8M9 1l-8 8" stroke="currentColor" stroke-width="1.2" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
    </div>

    <!-- Main workspace -->
    <div class="workspace">
      <aside class="sidebar">
        <ProcessList />
      </aside>
      <div class="main-col">
        <main class="main">
          <ProcessPanel />
        </main>
      </div>
    </div>
    <AppDialog />
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useProcessStore } from '@/stores/processStore'
import { useWindowActivity, windowActive } from '@/composables/useWindowActivity'
import { usePolling } from '@/composables/usePolling'
import ProcessList from '@/components/ProcessList.vue'
import ProcessPanel from '@/components/ProcessPanel.vue'
import AppDialog from '@/components/AppDialog.vue'

const store = useProcessStore()
const win = getCurrentWindow()
useWindowActivity()
const ready = ref(false)
usePolling(() => store.loadProcesses(), () => ready.value && windowActive.value)

const runningCount = computed(() =>
  store.processes.filter((p: any) => p.status === 'Running').length
)
usePolling(async () => store.updateUptimeClock(), () => windowActive.value && runningCount.value > 0, 1000)

const minimize = () => win.minimize()
const toggleMaximize = () => win.toggleMaximize()
const hideToTray = () => win.close()

onMounted(async () => {
  try {
    await store.loadProcesses()
    ready.value = true
  } finally {
    await nextTick()
    // Allow the initial Vue render to paint before revealing the native window.
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        invoke('main_window_ready').catch(console.error)
      })
    })
  }
})
</script>

<style>
* { box-sizing: border-box; margin: 0; padding: 0; }

html, body, #app {
  width: 100%;
  height: 100%;
  overflow: hidden;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI Variable', 'Segoe UI', system-ui, sans-serif;
  background: var(--bg-base);
  color: var(--text-primary);
  font-size: 14px;
}

.app {
  display: flex;
  flex-direction: column;
  width: 100%;
  height: 100vh;
  overflow: hidden;
  background: var(--bg-base);
}

/* ── Titlebar ── */
.titlebar {
  display: flex;
  align-items: center;
  height: 38px;
  background: var(--bg-surface);
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
  user-select: none;
}

.titlebar-left {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 14px;
  flex: 0 0 auto;
}

.app-logo { width: 16px; height: 16px; flex-shrink: 0; }

.app-name {
  font-size: var(--font-size-body);
  font-weight: 600;
  color: var(--text-secondary);
  letter-spacing: 0.01em;
  white-space: nowrap;
}

.titlebar-center {
  flex: 1;
  display: flex;
  justify-content: center;
}

.process-count {
  font-size: var(--font-size-small);
  color: var(--text-muted);
  letter-spacing: 0.02em;
}

.titlebar-controls {
  display: flex;
  align-items: stretch;
  height: 38px;
  flex-shrink: 0;
}

.wc {
  width: 46px;
  height: 38px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: none;
  color: var(--text-muted);
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}
.wc:hover { background: var(--bg-hover); color: var(--text-primary); }
.wc-close:hover { background: #c0392b !important; color: #fff !important; }

/* ── Workspace ── */
.workspace {
  display: flex;
  flex: 1;
  overflow: hidden;
  min-height: 0;
}

.sidebar {
  width: 290px;
  min-width: 220px;
  flex-shrink: 0;
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.main-col {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.main {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}

/* ── Scrollbar ── */
::-webkit-scrollbar { width: 5px; height: 5px; }
::-webkit-scrollbar-track { background: transparent; }
::-webkit-scrollbar-thumb { background: var(--scrollbar); border-radius: 3px; }
::-webkit-scrollbar-thumb:hover { background: var(--border-control-hover); }
</style>
