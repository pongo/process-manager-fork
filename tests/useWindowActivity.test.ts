import { afterEach, expect, mock, test } from 'bun:test'
import { createRenderer, nextTick } from 'vue'

let visible = false
let minimized = false
let subscriptionFails = false
mock.module('@tauri-apps/api/window', () => ({
  getCurrentWindow: () => ({
    isVisible: async () => visible,
    isMinimized: async () => minimized,
    onResized: async () => {
      if (subscriptionFails) throw new Error('Subscription failed')
      return () => {}
    },
    onFocusChanged: async () => () => {},
  }),
}))

const { useWindowActivity, windowActive } = await import('../src/composables/useWindowActivity')
const { usePolling } = await import('../src/composables/usePolling')
const renderer = createRenderer({
  createElement: () => ({}), createText: () => ({}), createComment: () => ({}),
  insert() {}, remove() {}, setText() {}, setElementText() {}, patchProp() {},
  parentNode: () => null, nextSibling: () => null,
})
let dispose = () => {}

function mountActivity(onPoll: () => Promise<void> = async () => {}) {
  let refresh!: () => Promise<void>
  const app = renderer.createApp({
    setup() {
      refresh = useWindowActivity()
      usePolling(onPoll, () => windowActive.value)
      return () => null
    },
  })
  app.mount({})
  dispose = () => app.unmount()
  return () => refresh()
}

afterEach(() => {
  dispose()
  visible = false
  minimized = false
  subscriptionFails = false
})

test('refreshes polling state after the initially hidden window is shown', async () => {
  globalThis.document = { hidden: false, addEventListener() {}, removeEventListener() {} } as unknown as Document
  let polls = 0
  const refresh = mountActivity(async () => { polls++ })
  await nextTick()
  expect(windowActive.value).toBe(false)
  visible = true
  await refresh()
  expect(windowActive.value).toBe(true)
  await nextTick()
  expect(polls).toBe(1)
  minimized = true
  await refresh()
  expect(windowActive.value).toBe(false)
  minimized = false
  await refresh()
  expect(windowActive.value).toBe(true)
  await nextTick()
  expect(polls).toBe(2)
  dispose()
  await refresh()
  expect(windowActive.value).toBe(false)
})

test('initial visibility is checked even if event subscription fails', async () => {
  globalThis.document = { hidden: false, addEventListener() {}, removeEventListener() {} } as unknown as Document
  visible = true
  subscriptionFails = true
  mountActivity()
  await new Promise(resolve => setTimeout(resolve, 0))
  expect(windowActive.value).toBe(true)
})
