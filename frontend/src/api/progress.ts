import { client } from './client'
import type { ReadingProgressRecord } from '@/types/api/ReadingProgressRecord'

export interface ProgressUpdate {
  chapter_order: number
  page_index: number
}

export const progressApi = {
  get: (bookId: number) =>
    client.get<ReadingProgressRecord | null>(`/api/progress/${bookId}`),
  update: (bookId: number, data: ProgressUpdate) =>
    client.put(`/api/progress/${bookId}`, data),
}
