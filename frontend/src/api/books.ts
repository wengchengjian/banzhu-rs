import { client } from './client'
import type { Paginated } from '@/types/api'
import type {
  BookListItem,
  BookDetail,
  ChaptersResponse,
  ChapterContent,
  DeleteResult,
} from '@/types/books'

export interface ListBooksParams {
  page?: number
  limit?: number
  category?: string
}

export const booksApi = {
  list: (params: ListBooksParams = {}) => {
    const qs = new URLSearchParams()
    if (params.page) qs.set('page', String(params.page))
    if (params.limit) qs.set('limit', String(params.limit))
    if (params.category) qs.set('category', params.category)
    const query = qs.toString()
    return client.get<Paginated<BookListItem>>(`/api/books${query ? `?${query}` : ''}`)
  },
  get: (id: number) => client.get<BookDetail>(`/api/books/${id}`),
  chapters: (id: number) => client.get<ChaptersResponse>(`/api/books/${id}/chapters`),
  chapterContent: (bookId: number, chapterOrder: number) =>
    client.get<ChapterContent>(`/api/books/${bookId}/chapters/${chapterOrder}`),
  delete: (id: number) => client.delete<DeleteResult>(`/api/books/${id}`),
  // 文件下载不走 JSON client，直接触发浏览器下载
  exportBook: (id: number, format: 'txt' | 'epub') => {
    const a = document.createElement('a')
    a.href = `/api/export/${id}?format=${format}`
    a.download = ''
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
  },
}
