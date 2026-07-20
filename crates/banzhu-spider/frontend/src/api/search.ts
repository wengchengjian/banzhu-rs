import { client } from './client'

/// 搜索结果项（GET /api/search 返回的 items 元素）
export interface SearchResult {
  book_id: number
  title: string
  author: string
  category: string
  word_count: number
  relevance_score: number
  snippet: string
  created_at: number
}

/// 搜索响应
export interface SearchResponse {
  items: SearchResult[]
  total: number
  page: number
  limit: number
}

/// 搜索字段范围
export type SearchField = 'all' | 'title' | 'author' | 'content'

/// 搜索参数
export interface SearchParams {
  q: string
  field?: SearchField
  page?: number
  limit?: number
  exact?: boolean
}

export const searchApi = {
  search: (params: SearchParams) => {
    const qs = new URLSearchParams()
    qs.set('q', params.q)
    if (params.field && params.field !== 'all') qs.set('field', params.field)
    if (params.page) qs.set('page', String(params.page))
    if (params.limit) qs.set('limit', String(params.limit))
    if (params.exact) qs.set('exact', '1')
    return client.get<SearchResponse>(`/api/search?${qs.toString()}`)
  },
}
