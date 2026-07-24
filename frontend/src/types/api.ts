export type { BookRecord } from './api/BookRecord'
export type { ChapterRecord } from './api/ChapterRecord'
export type { SectionRecord } from './api/SectionRecord'
export type { BookshelfRecord } from './api/BookshelfRecord'
export type { ReadingProgressRecord } from './api/ReadingProgressRecord'
export type { CrawlLogRecord } from './api/CrawlLogRecord'
export type { CrawlTaskRecord } from './api/CrawlTaskRecord'
export type { ReadingSessionRecord } from './api/ReadingSessionRecord'
export type { ReadingGoalRecord } from './api/ReadingGoalRecord'

export interface ApiResponse<T> { code: number; data?: T; msg?: string }
export interface Paginated<T> { items: T[]; total: number; page: number; limit: number }
