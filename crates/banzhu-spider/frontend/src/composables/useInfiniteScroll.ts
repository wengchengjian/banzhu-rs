import { ref, onMounted, onUnmounted, watch, type Ref } from 'vue'

interface Options {
  /** 触发距离（px），sentinel 距视口底部多远时触发 */
  rootMargin?: string
  /** 加载更多数据的函数，返回 false 表示没有更多 */
  loadMore: () => Promise<boolean | void>
  /** 是否启用（默认 true） */
  enabled?: Ref<boolean>
}

/**
 * 无限滚动 composable
 *
 * 用法：
 * ```vue
 * const { sentinel, loading, hasMore, reset, check } = useInfiniteScroll({
 *   loadMore: async () => {
 *     const result = await fetchData(page++)
 *     if (result.length === 0) return false
 *   }
 * })
 * ```
 */
export function useInfiniteScroll(opts: Options) {
  const sentinel = ref<HTMLElement | null>(null)
  const loading = ref(false)
  const hasMore = ref(true)
  let observer: IntersectionObserver | null = null
  // 用于丢弃 reset 之后才返回的过期请求结果
  let loadVersion = 0

  async function check() {
    if (loading.value || !hasMore.value) return
    if (opts.enabled && !opts.enabled.value) return
    loading.value = true
    const currentVersion = loadVersion
    try {
      const result = await opts.loadMore()
      // 版本不匹配说明期间发生过 reset，丢弃过期结果
      if (currentVersion !== loadVersion) return
      if (result === false) hasMore.value = false
    } finally {
      // 仅当此次请求未被 reset 失效时才释放 loading
      if (currentVersion === loadVersion) loading.value = false
    }
  }

  onMounted(() => {
    observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) check()
      },
      { rootMargin: opts.rootMargin ?? '200px' },
    )
    // 不在此处 observe，由 watch(sentinel) 处理：
    // sentinel 可能在 v-if/v-else 切换中延迟挂载，onMounted 时未必已存在
  })

  // 监听 sentinel 引用变化，元素挂载时 observe，卸载时 unobserve
  watch(sentinel, (el, _oldEl, onCleanup) => {
    if (!observer) return
    if (el) {
      observer.observe(el)
      onCleanup(() => {
        observer?.unobserve(el)
      })
    }
  })

  onUnmounted(() => {
    observer?.disconnect()
    observer = null
  })

  /** 重置状态（切换分类时调用） */
  function reset() {
    // 让 in-flight 请求结果失效，避免覆盖新分类的状态
    loadVersion++
    hasMore.value = true
    loading.value = false
  }

  return { sentinel, loading, hasMore, reset, check }
}
