/// 书籍列表项（GET /api/books 返回的 items 元素）
export interface BookListItem {
  id: number
  title: string
  author: string
  category: string
  word_count: number
  likes: number
  chapter_count: number
  created_at: number
}

/// 书籍详情（GET /api/books/:id 返回）
export interface BookDetail {
  id: number
  title: string
  author: string
  category: string
  introduce: string
  word_count: number
  likes: number
  chapter_count: number
  status: string // "连载中" | "已完结"
  created_at: number
}

/// 章节列表项（GET /api/books/:id/chapters 返回的 items 元素）
export interface ChapterListItem {
  id: number
  title: string
  order: number
}

/// 章节列表响应
export interface ChaptersResponse {
  items: ChapterListItem[]
  total: number
}

/// 章节内容（GET /api/books/:id/chapters/:order 返回）
export interface ChapterContent {
  chapter_id: number
  title: string
  order: number
  book_id: number
  content: string
  prev_order: number | null
  next_order: number | null
}

/// 删除结果
export interface DeleteResult {
  deleted: number
}
