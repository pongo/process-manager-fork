import { onMounted, onUnmounted, readonly, ref } from 'vue'
import { getCurrentWindow } from '@tauri-apps/api/window'
import type { UnlistenFn } from '@tauri-apps/api/event'

const active = ref(false)
export const windowActive = readonly(active)

export function useWindowActivity() {
  const win = getCurrentWindow()
  const unlisteners: UnlistenFn[] = []
  let disposed = false
  let revision = 0

  const update = async () => {
    if (disposed) return
    const current = ++revision
    if (document.hidden) active.value = false
    try {
      const [visible, minimized] = await Promise.all([win.isVisible(), win.isMinimized()])
      if (!disposed && current === revision) {
        active.value = !document.hidden && visible && !minimized
      }
    } catch (error) {
      console.error('Failed to read window visibility:', error)
    }
  }

  onMounted(async () => {
    document.addEventListener('visibilitychange', update)
    await update()
    for (const subscribe of [
      () => win.onResized(update),
      () => win.onFocusChanged(update),
    ]) {
      if (disposed) return
      try {
        const unlisten = await subscribe()
        if (disposed) unlisten()
        else unlisteners.push(unlisten)
      } catch (error) {
        console.error('Failed to subscribe to window activity:', error)
      }
    }
    if (!disposed) await update()
  })

  onUnmounted(() => {
    disposed = true
    revision++
    active.value = false
    document.removeEventListener('visibilitychange', update)
    unlisteners.forEach((unlisten) => unlisten())
  })

  return update
}
