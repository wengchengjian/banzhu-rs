import { ref, onUnmounted } from 'vue'

export interface SSEOptions {
  url: string
  /** 重连间隔（毫秒），默认 3000 */
  reconnectInterval?: number
  /** 最大重试次数，默认 3 */
  maxRetries?: number
}

/**
 * 封装 EventSource，支持自动重连与事件分发。
 *
 * 用法：
 * ```ts
 * const { connected, error, connect, on, close } = useSSE({ url: '/api/crawl/stream' })
 * on('status', (data) => { ... })
 * on('task:update', (data) => { ... })
 * on('task:full', (data) => { ... })
 * on('log', (data) => { ... })
 * connect()
 * ```
 */
export function useSSE(options: SSEOptions) {
  const { url, reconnectInterval = 3000, maxRetries = 3 } = options
  const connected = ref(false)
  const error = ref<string | null>(null)

  let eventSource: EventSource | null = null
  let retryCount = 0
  let retryTimer: number | null = null

  // 事件名 → 处理函数列表；重连时需要把这些 handler 重新挂到新 EventSource 上
  const handlers = new Map<string, ((data: unknown) => void)[]>()

  function attach(event: string, handler: (data: unknown) => void) {
    eventSource?.addEventListener(event, (e: MessageEvent) => {
      try {
        handler(JSON.parse(e.data))
      } catch {
        handler(e.data)
      }
    })
  }

  function connect() {
    if (eventSource) close()
    eventSource = new EventSource(url)

    eventSource.onopen = () => {
      connected.value = true
      error.value = null
      retryCount = 0
    }

    eventSource.onerror = () => {
      connected.value = false
      error.value = 'SSE 连接失败'
      eventSource?.close()
      eventSource = null

      retryCount++
      if (retryCount <= maxRetries) {
        retryTimer = window.setTimeout(() => connect(), reconnectInterval)
      } else {
        error.value = `SSE 连接失败，已重试 ${maxRetries} 次`
      }
    }

    // 把已注册的 handler 重新挂到新的 EventSource 上
    for (const event of handlers.keys()) {
      for (const fn of handlers.get(event) ?? []) {
        attach(event, fn)
      }
    }
  }

  function on(event: string, handler: (data: any) => void) {
    if (!handlers.has(event)) handlers.set(event, [])
    handlers.get(event)!.push(handler)
    attach(event, handler)
  }

  function close() {
    if (retryTimer != null) {
      clearTimeout(retryTimer)
      retryTimer = null
    }
    eventSource?.close()
    eventSource = null
    connected.value = false
  }

  onUnmounted(() => close())

  return { connected, error, connect, on, close }
}
