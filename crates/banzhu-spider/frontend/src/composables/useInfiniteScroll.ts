import { ref, onMounted, onUnmounted, type Ref } from 'vue'

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

  async function check() {
    if (loading.value || !hasMore.value) return
    if (opts.enabled && !opts.enabled.value) return
    loading.value = true
    try {
      const result = await opts.loadMore()
      if (result === false) hasMore.value = false
    } finally {
      loading.value = false
    }
  }

  onMounted(() => {
    observer = new IntersectionObserver(
      (entries) => {
        if (entries[0]?.isIntersecting) check()
      },
      { rootMargin: opts.rootMargin ?? '200px' },
    )
    if (sentinel.value) observer.observe(sentinel.value)
  })

  onUnmounted(() => {
    observer?.disconnect()
    observer = null
  })

  /** 重置状态（切换分类时调用） */
  function reset() {
    hasMore.value = true
    loading.value = false
  }

  return { sentinel, loading, hasMore, reset, check }
}
