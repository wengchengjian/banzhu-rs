import { describe, it, expect, beforeEach, vi } from 'vitest'
import 'fake-indexeddb/auto'

const DB_NAME = 'banzhu-reader'

// 必须在 import 'fake-indexeddb/auto' 之后动态 import useChapterCache
// 因为 useChapterCache 内部用模块级 dbPromise 单例，需要在每个测试前重置模块
let useChapterCache: typeof import('../useChapterCache').useChapterCache
let clearAll: () => Promise<void>

beforeEach(async () => {
  vi.resetModules()
  // 删除 fake-indexeddb 的数据库实例，确保每个测试从空库开始
  await new Promise<void>((resolve, reject) => {
    const req = indexedDB.deleteDatabase(DB_NAME)
    req.onsuccess = () => resolve()
    req.onerror = () => reject(req.error)
    req.onblocked = () => resolve()
  })
  const mod = await import('../useChapterCache')
  useChapterCache = mod.useChapterCache
  // 在测试开始前确保 store 是干净的
  const cache = useChapterCache()
  clearAll = cache.clearAll
  await clearAll()
})

describe('useChapterCache', () => {
  it('put 后 get 能取回数据', async () => {
    const cache = useChapterCache()
    await cache.put({
      bookId: 1,
      chapterOrder: 5,
      title: '第五章',
      content: '内容',
      cachedAt: Date.now(),
    })
    // 实际 API 返回 undefined 而不是 null（当记录不存在时）
    const v = await cache.get(1, 5)
    expect(v).toBeDefined()
    expect(v!.title).toBe('第五章')
    expect(v!.content).toBe('内容')
  })

  it('deleteBook 删除该书所有章节', async () => {
    const cache = useChapterCache()
    await cache.put({ bookId: 1, chapterOrder: 1, title: '1', content: 'x', cachedAt: 0 })
    await cache.put({ bookId: 1, chapterOrder: 2, title: '2', content: 'x', cachedAt: 0 })
    await cache.put({ bookId: 2, chapterOrder: 1, title: '3', content: 'x', cachedAt: 0 })
    await cache.deleteBook(1)
    expect(await cache.get(1, 1)).toBeUndefined()
    expect(await cache.get(1, 2)).toBeUndefined()
    expect(await cache.get(2, 1)).toBeDefined()
  })

  it('getBookCount 返回章节总数', async () => {
    const cache = useChapterCache()
    await cache.put({ bookId: 1, chapterOrder: 1, title: '1', content: 'x', cachedAt: 0 })
    await cache.put({ bookId: 1, chapterOrder: 2, title: '2', content: 'x', cachedAt: 0 })
    expect(await cache.getBookCount(1)).toBe(2)
    // 其他书没有章节
    expect(await cache.getBookCount(2)).toBe(0)
  })

  it('clearAll 清空所有章节', async () => {
    const cache = useChapterCache()
    await cache.put({ bookId: 1, chapterOrder: 1, title: '1', content: 'x', cachedAt: 0 })
    await cache.put({ bookId: 2, chapterOrder: 1, title: '2', content: 'x', cachedAt: 0 })
    await cache.clearAll()
    expect(await cache.get(1, 1)).toBeUndefined()
    expect(await cache.get(2, 1)).toBeUndefined()
    expect(await cache.getBookCount(1)).toBe(0)
  })

  it('get 不存在的章节返回 undefined', async () => {
    const cache = useChapterCache()
    expect(await cache.get(999, 999)).toBeUndefined()
  })
})
