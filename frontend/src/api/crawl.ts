import { client } from './client'
import type { CrawlTaskRecord, CrawlLogRecord } from '@/types/api'

// ─── 类型定义（与后端 src/web/crawl.rs + src/scheduler.rs 对齐） ───────────────

/** 爬虫整体状态：对应 GET /api/crawl/status 与 SSE `status` 事件 */
export interface CrawlStatus {
  running: boolean
  current_page: number
  pages_limit: number
  books_found: number
  books_downloaded: number
  books_failed: number
  books_skipped: number
  /** SSE 事件不携带此字段，故设为可选 */
  last_run?: string
  message: string
}

/** 单本书的爬取任务记录 */
export type CrawlTask = CrawlTaskRecord

/** 爬取日志条目（/api/crawl/logs 返回） */
export type CrawlLog = CrawlLogRecord

/** SSE `log` 事件 payload（注意字段名是 timestamp，不是 created_at） */
export interface SSELogEvent {
  id: number
  level: string
  message: string
  timestamp: number
}

/** SSE `task:full` 事件 payload */
export interface SSETaskFullEvent {
  tasks: CrawlTask[]
}

/** SSE `task:update` 事件 payload */
export interface SSETaskUpdateEvent {
  task: CrawlTask
}

/** /api/crawl/tasks 返回的 status_count 子对象 */
export interface CrawlStatusCount {
  pending: number
  running: number
  success: number
  failed: number
  skipped: number
  total: number
}

/** /api/crawl/tasks 返回结构 */
export interface CrawlTasksResponse {
  items: CrawlTask[]
  total: number
  page: number
  limit: number
  status_count: CrawlStatusCount
}

/** /api/crawl/tasks 查询参数 */
export interface CrawlTasksParams {
  page?: number
  limit?: number
  status?: string
}

/** /api/crawl/manual 返回 */
export interface CrawlManualResult {
  message: string
  book_id: number
}

/** /api/crawl/retry-failed 与 /api/crawl/tasks DELETE 返回 */
export interface CrawlAffectedResult {
  count: number
}

// ─── crawlApi ────────────────────────────────────────────────────────────────

export const crawlApi = {
  /** GET /api/crawl/status — 获取爬虫整体状态 */
  status: () => client.get<CrawlStatus>('/api/crawl/status'),

  /** GET /api/crawl/tasks — 分页获取爬取任务列表 */
  tasks: (params: CrawlTasksParams = {}) => {
    const qs = new URLSearchParams()
    if (params.page != null) qs.set('page', String(params.page))
    if (params.limit != null) qs.set('limit', String(params.limit))
    if (params.status) qs.set('status', params.status)
    const query = qs.toString()
    return client.get<CrawlTasksResponse>(`/api/crawl/tasks${query ? `?${query}` : ''}`)
  },

  /** GET /api/crawl/logs?limit=N — 获取最近 N 条日志 */
  logs: (limit = 100) =>
    client.get<CrawlLog[]>(`/api/crawl/logs?limit=${limit}`),

  /** POST /api/crawl/manual { url } — 手动触发指定书籍 URL 的爬取 */
  manual: (url: string) =>
    client.post<CrawlManualResult>('/api/crawl/manual', { url }),

  /** POST /api/crawl/full — 手动触发全量爬取（爬完所有列表页） */
  full: () =>
    client.post<{ message: string }>('/api/crawl/full', {}),

  /** POST /api/crawl/retry-failed — 批量重试所有 failed 状态任务 */
  retryFailed: () =>
    client.post<CrawlAffectedResult>('/api/crawl/retry-failed'),

  /** DELETE /api/crawl/tasks?status=xxx — 按状态删除任务 */
  deleteByStatus: (status: string) =>
    client.delete<CrawlAffectedResult>(`/api/crawl/tasks?status=${encodeURIComponent(status)}`),
}
