<template>
  <teleport to="body">
    <transition name="dialog-fade">
      <div v-if="state.show" class="dialog-overlay" @mousedown.self="onBackdrop">
        <div :class="['dialog-card', `dialog-${state.type}`]" role="dialog" aria-modal="true">
          <!-- Icon -->
          <div class="dialog-icon-wrap">
            <svg v-if="state.type === 'alert'" viewBox="0 0 20 20" width="20" height="20" fill="none">
              <circle cx="10" cy="10" r="8.5" stroke="currentColor" stroke-width="1.5"/>
              <path d="M10 6v4.5" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
              <circle cx="10" cy="13.5" r="0.9" fill="currentColor"/>
            </svg>
            <svg v-else-if="state.type === 'danger'" viewBox="0 0 20 20" width="20" height="20" fill="none">
              <path d="M10 2L18.5 17H1.5L10 2z" stroke="currentColor" stroke-width="1.5" stroke-linejoin="round"/>
              <path d="M10 8v4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"/>
              <circle cx="10" cy="14.5" r="0.9" fill="currentColor"/>
            </svg>
            <svg v-else viewBox="0 0 20 20" width="20" height="20" fill="none">
              <circle cx="10" cy="10" r="8.5" stroke="currentColor" stroke-width="1.5"/>
              <path d="M7 10l2 2 4-4" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
            </svg>
          </div>

          <div class="dialog-body">
            <p class="dialog-title">{{ state.title }}</p>
            <p class="dialog-msg">{{ state.message }}</p>
          </div>

          <div class="dialog-actions">
            <button
              v-if="state.cancelLabel"
              class="dialog-btn cancel"
              @click="dismiss(false)"
            >{{ state.cancelLabel }}</button>
            <button
              :class="['dialog-btn', 'confirm', state.type]"
              @click="dismiss(true)"
              ref="confirmBtn"
            >{{ state.confirmLabel }}</button>
          </div>
        </div>
      </div>
    </transition>
  </teleport>
</template>

<script setup lang="ts">
import { watch, nextTick, ref } from 'vue'
import { useDialog } from '@/composables/useDialog'

const { state, dismiss } = useDialog()
const confirmBtn = ref<HTMLButtonElement>()

// auto-focus confirm button when dialog opens
watch(() => state.show, async (v) => {
  if (v) { await nextTick(); confirmBtn.value?.focus() }
})

// close on Escape
const onKey = (e: KeyboardEvent) => {
  if (!state.show) return
  if (e.key === 'Escape') dismiss(false)
  if (e.key === 'Enter') dismiss(true)
}
window.addEventListener('keydown', onKey)

// backdrop only closes alert, not confirm
const onBackdrop = () => {
  if (state.type === 'alert') dismiss(true)
}
</script>

<style scoped>
.dialog-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.65);
  backdrop-filter: blur(3px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.dialog-card {
  background: var(--bg-surface);
  border: 1px solid var(--border-control);
  border-radius: 12px;
  width: 340px;
  max-width: calc(100vw - 32px);
  max-height: calc(100vh - 32px);
  overflow-y: auto;
  padding: 20px;
  display: flex;
  flex-direction: column;
  gap: 14px;
  box-shadow: 0 24px 64px rgba(0, 0, 0, 0.7), 0 0 0 1px rgba(255,255,255,0.03);
}

.dialog-icon-wrap {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 10px;
  flex-shrink: 0;
  align-self: flex-start;
}
.dialog-alert .dialog-icon-wrap { background: var(--info-bg); color: var(--info); border: 1px solid var(--info-border); }
.dialog-confirm .dialog-icon-wrap { background: var(--success-bg); color: var(--success); border: 1px solid var(--success-border); }
.dialog-danger .dialog-icon-wrap { background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger-border); }

.dialog-body { display: flex; flex-direction: column; gap: 5px; }
.dialog-title {
  font-size: var(--font-size-title);
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}
.dialog-msg {
  font-size: var(--font-size-body);
  color: var(--text-secondary);
  line-height: 1.5;
}

.dialog-actions {
  display: flex;
  gap: 8px;
  justify-content: flex-end;
  margin-top: 4px;
}

.dialog-btn {
  padding: 7px 18px;
  border-radius: 7px;
  font-size: var(--font-size-body);
  font-weight: 600;
  cursor: pointer;
  border: 1px solid transparent;
  transition: background 0.12s, border-color 0.12s, color 0.12s;
  outline: none;
}
.dialog-btn:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }

.dialog-btn.cancel {
  background: var(--bg-raised);
  border-color: var(--border-control);
  color: var(--text-secondary);
}
.dialog-btn.cancel:hover { background: var(--bg-hover); color: var(--text-secondary); border-color: var(--border-control-hover); }

.dialog-btn.confirm.alert,
.dialog-btn.confirm.confirm {
  background: var(--accent-bg);
  border-color: var(--accent-border);
  color: var(--accent);
}
.dialog-btn.confirm.alert:hover,
.dialog-btn.confirm.confirm:hover { background: var(--accent-bg-hover); border-color: var(--accent); color: var(--accent-hover); }

.dialog-btn.confirm.danger {
  background: var(--danger-bg-hover);
  border-color: var(--danger-border);
  color: var(--danger-hover);
}
.dialog-btn.confirm.danger:hover { background: var(--danger-bg-hover); border-color: var(--danger-border); color: #fecaca; }

/* ── Animations ── */
.dialog-fade-enter-active { transition: opacity 0.15s, transform 0.15s; }
.dialog-fade-leave-active { transition: opacity 0.12s, transform 0.1s; }
.dialog-fade-enter-from { opacity: 0; transform: scale(0.95); }
.dialog-fade-leave-to { opacity: 0; transform: scale(0.97); }
</style>
