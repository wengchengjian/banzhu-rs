import { defineStore } from 'pinia'
import { ref } from 'vue'
import { shelfApi } from '@/api/shelf'
import { booksApi } from '@/api/books'
import type { BookshelfRecord } from '@/types/api/BookshelfRecord'
import type { BookDetail } from '@/types/books'

export interface ShelfItem {
  shelf: BookshelfRecord
  book: BookDetail
}

export const useShelfStore = defineStore('shelf', () => {
  const items = ref<ShelfItem[]>([])
  const loading = ref(false)
  const errorMsg = ref('')

  async function load(group?: string) {
    loading.value = true
    errorMsg.value = ''
    try {
      const shelfList = await shelfApi.list(group)
      // 并行获取每本书的详情
      const books = await Promise.all(
        shelfList.map(s => booksApi.get(s.book_id).catch(() => null)),
      )
      items.value = shelfList
        .map((shelf, i) => ({ shelf, book: books[i] }))
        .filter((item): item is ShelfItem => item.book !== null)
    } catch (e) {
      errorMsg.value = (e as Error).message
      items.value = []
    } finally {
      loading.value = false
    }
  }

  async function add(bookId: number, group?: string) {
    await shelfApi.add(bookId, group)
    await load()
  }

  async function move(bookId: number, group: string) {
    await shelfApi.move(bookId, group)
    await load()
  }

  async function remove(bookId: number) {
    await shelfApi.remove(bookId)
    items.value = items.value.filter(i => i.shelf.book_id !== bookId)
  }

  return { items, loading, errorMsg, load, add, move, remove }
})
