const DB_NAME = 'banzhu-reader'
const DB_VERSION = 1
const STORE = 'chapters'

export interface CachedChapter {
  bookId: number
  chapterOrder: number
  title: string
  content: string
  cachedAt: number
}

let dbPromise: Promise<IDBDatabase> | null = null

function openDB(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise
  dbPromise = new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION)
    req.onupgradeneeded = () => {
      const db = req.result
      if (!db.objectStoreNames.contains(STORE)) {
        const store = db.createObjectStore(STORE, {
          keyPath: ['bookId', 'chapterOrder'],
        })
        store.createIndex('by_book', 'bookId', { unique: false })
        store.createIndex('by_cached_at', 'cachedAt', { unique: false })
      }
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
  return dbPromise
}

function tx<T>(
  mode: IDBTransactionMode,
  fn: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  return openDB().then(
    db =>
      new Promise<T>((resolve, reject) => {
        const t = db.transaction(STORE, mode)
        const req = fn(t.objectStore(STORE))
        req.onsuccess = () => resolve(req.result)
        req.onerror = () => reject(req.error)
      }),
  )
}

export function useChapterCache() {
  async function get(
    bookId: number,
    chapterOrder: number,
  ): Promise<CachedChapter | undefined> {
    return tx<CachedChapter>('readonly', s =>
      s.get([bookId, chapterOrder]) as IDBRequest<CachedChapter>,
    )
  }

  async function put(chapter: CachedChapter): Promise<void> {
    await tx('readwrite', s => s.put(chapter))
  }

  async function deleteBook(bookId: number): Promise<void> {
    const db = await openDB()
    await new Promise<void>((resolve, reject) => {
      const t = db.transaction(STORE, 'readwrite')
      const idx = t.objectStore(STORE).index('by_book')
      const cursorReq = idx.openCursor(IDBKeyRange.only(bookId))
      cursorReq.onsuccess = () => {
        const cursor = cursorReq.result
        if (cursor) {
          cursor.delete()
          cursor.continue()
        }
      }
      t.oncomplete = () => resolve()
      t.onerror = () => reject(t.error)
    })
  }

  async function clearAll(): Promise<void> {
    await tx('readwrite', s => s.clear())
  }

  async function getBookCount(bookId: number): Promise<number> {
    const db = await openDB()
    return new Promise<number>((resolve, reject) => {
      const t = db.transaction(STORE, 'readonly')
      const idx = t.objectStore(STORE).index('by_book')
      const countReq = idx.count(IDBKeyRange.only(bookId))
      countReq.onsuccess = () => resolve(countReq.result)
      countReq.onerror = () => reject(countReq.error)
    })
  }

  async function estimateSize(): Promise<{ usage: number; quota: number }> {
    if ('storage' in navigator && 'estimate' in navigator.storage) {
      const est = await navigator.storage.estimate()
      return { usage: est.usage ?? 0, quota: est.quota ?? 0 }
    }
    return { usage: 0, quota: 0 }
  }

  async function refreshSize(): Promise<{ usage: number; quota: number }> {
    return estimateSize()
  }

  return {
    get,
    put,
    deleteBook,
    clearAll,
    getBookCount,
    estimateSize,
    refreshSize,
  }
}
