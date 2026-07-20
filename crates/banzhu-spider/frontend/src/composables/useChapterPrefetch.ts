import { useChapterCache } from './useChapterCache'

/**
 * 章节预加载：在用户阅读当前章节时，后台预取下一 N 章并缓存到 IndexedDB。
 * 使用 requestIdleCallback（不支持时回退 setTimeout）避免抢占主线程。
 */
export function useChapterPrefetch() {
  const cache = useChapterCache()

  function prefetch(bookId: number, currentOrder: number, count = 3): void {
    if ('requestIdleCallback' in window) {
      const ric = (window as Window).requestIdleCallback
      ric(() => {
        void doPrefetch(bookId, currentOrder, count)
      })
    } else {
      setTimeout(() => {
        void doPrefetch(bookId, currentOrder, count)
      }, 1000)
    }
  }

  async function doPrefetch(
    bookId: number,
    currentOrder: number,
    count: number,
  ): Promise<void> {
    for (let i = 1; i <= count; i++) {
      const order = currentOrder + i
      // 已缓存则跳过
      const cached = await cache.get(bookId, order)
      if (cached) continue
      try {
        const res = await fetch(`/api/books/${bookId}/chapters/${order}`)
        if (!res.ok) continue
        const data = await res.json()
        // 后端响应格式：{ code: 0, data: { title, content, ... } }
        if (data.code !== 0 || !data.data) continue
        await cache.put({
          bookId,
          chapterOrder: order,
          title: data.data.title,
          content: data.data.content,
          cachedAt: Date.now(),
        })
      } catch {
        // 静默失败，不影响用户阅读
      }
    }
  }

  return { prefetch }
}
