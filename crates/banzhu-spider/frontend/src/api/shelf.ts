import { client } from './client'
import type { BookshelfRecord } from '@/types/api/BookshelfRecord'

export type ShelfGroup = 'reading' | 'want' | 'finished'

export const shelfApi = {
  list: (group?: string) =>
    client.get<BookshelfRecord[]>(`/api/bookshelf${group ? `?group=${encodeURIComponent(group)}` : ''}`),
  add: (bookId: number, group?: string) =>
    client.post('/api/bookshelf', { book_id: bookId, group }),
  move: (bookId: number, group: string) =>
    client.put(`/api/bookshelf/${bookId}`, { group }),
  remove: (bookId: number) =>
    client.delete(`/api/bookshelf/${bookId}`),
}
