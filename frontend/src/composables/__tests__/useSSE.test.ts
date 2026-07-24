import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest'
import { effectScope } from 'vue'
import { useSSE } from '../useSSE'

class MockEventSource {
  static instances: MockEventSource[] = []
  url: string
  listeners: Record<string, EventListener[]> = {}
  onopen: (() => void) | null = null
  onerror: (() => void) | null = null
  readyState = 0
  closed = false
  closeSpy = vi.fn(() => {
    this.closed = true
    this.readyState = 2
  })

  constructor(url: string) {
    this.url = url
    MockEventSource.instances.push(this)
  }

  addEventListener(type: string, fn: EventListener): void {
    (this.listeners[type] ||= []).push(fn)
  }

  removeEventListener(): void {}

  close(): void {
    this.closeSpy()
  }

  emit(type: string, data: unknown): void {
    const evt = { data: JSON.stringify(data) } as MessageEvent
    ;(this.listeners[type] || []).forEach((fn) => fn(evt))
  }
}

const originalEventSource = globalThis.EventSource

beforeEach(() => {
  MockEventSource.instances = []
  // @ts-expect-error 注入 mock
  globalThis.EventSource = MockEventSource
})

afterEach(() => {
  globalThis.EventSource = originalEventSource
  MockEventSource.instances = []
})

describe('useSSE', () => {
  it('connect() 后 on 注册的 handler 能收到事件', () => {
    const scope = effectScope()
    const sse = scope.run(() => useSSE({ url: '/api/crawl/stream' }))!
    const handler = vi.fn()
    sse.on('log', handler)
    sse.connect()

    const instance = MockEventSource.instances[0]
    expect(instance).toBeDefined()
    instance.emit('log', { msg: 'hello' })

    expect(handler).toHaveBeenCalledTimes(1)
    expect(handler).toHaveBeenCalledWith({ msg: 'hello' })
    scope.stop()
  })

  it('close() 后 EventSource 被关闭', () => {
    const scope = effectScope()
    const sse = scope.run(() => useSSE({ url: '/api/crawl/stream' }))!
    sse.connect()
    const instance = MockEventSource.instances[0]
    expect(instance).toBeDefined()

    sse.close()
    expect(instance.closed).toBe(true)
    expect(instance.readyState).toBe(2)
    expect(instance.closeSpy).toHaveBeenCalled()
    scope.stop()
  })

  it('onopen 时 connected 变为 true', () => {
    const scope = effectScope()
    const sse = scope.run(() => useSSE({ url: '/api/crawl/stream' }))!
    sse.connect()
    const instance = MockEventSource.instances[0]

    expect(sse.connected.value).toBe(false)
    instance.onopen?.()
    expect(sse.connected.value).toBe(true)
    expect(sse.error.value).toBeNull()
    scope.stop()
  })

  it('onerror 时 connected 变为 false 且 error 有值', () => {
    const scope = effectScope()
    const sse = scope.run(
      () => useSSE({ url: '/api/crawl/stream', reconnectInterval: 0, maxRetries: 0 }),
    )!
    sse.connect()
    const instance = MockEventSource.instances[0]

    // 先打开，再触发错误
    instance.onopen?.()
    expect(sse.connected.value).toBe(true)

    instance.onerror?.()
    expect(sse.connected.value).toBe(false)
    expect(sse.error.value).not.toBeNull()
    expect(sse.error.value).not.toBe('')
    scope.stop()
  })
})
