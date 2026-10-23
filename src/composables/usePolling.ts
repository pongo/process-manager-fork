import { onUnmounted, watch, type WatchSource } from 'vue'

export function usePolling(task: () => Promise<void>, enabled: WatchSource<boolean | string>, interval = 2000) {
  let timer: ReturnType<typeof setTimeout> | undefined
  let running = false
  let active = false
  let revision = 0

  const poll = async () => {
    if (!active || running) return
    running = true
    const current = revision
    try {
      await task()
    } catch (error) {
      console.error('Polling failed:', error)
    } finally {
      running = false
      if (active) timer = setTimeout(poll, current === revision ? interval : 0)
    }
  }

  const stop = watch(enabled, (value) => {
    active = !!value
    revision++
    clearTimeout(timer)
    if (active) void poll()
  }, { immediate: true })

  onUnmounted(() => {
    stop()
    active = false
    clearTimeout(timer)
  })
}
