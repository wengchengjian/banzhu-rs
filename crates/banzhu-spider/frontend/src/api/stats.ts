import { client } from './client'
import type { ReadingGoalRecord } from '@/types/api/ReadingGoalRecord'

export interface HeatmapPoint {
  date: string
  duration_sec: number
  chapters_read: number
}
export interface TimelinePoint {
  date: string
  duration_sec: number
  chapters_read: number
}
export interface TodayReading {
  duration_sec: number
  chapters_read: number
}
export interface ReadingHistoryItem {
  book_id: number
  book_title: string
  last_read_at: number
  last_chapter_order: number
  total_duration_sec: number
  total_chapters: number
}
export interface ReportSessionBody {
  book_id: number
  chapter_order: number
  duration_sec: number
  chapters_read: number
  started_at: number
  ended_at: number
}
export interface ReportSessionResult {
  ok: boolean
  session_id: number
}

export const statsApi = {
  heatmap: (year?: number) =>
    client.get<{ items: HeatmapPoint[] }>(`/api/stats/heatmap${year ? `?year=${year}` : ''}`),
  timeline: (days?: number) =>
    client.get<{ items: TimelinePoint[] }>(`/api/stats/reading-timeline${days ? `?days=${days}` : ''}`),
  reportSession: (data: ReportSessionBody) =>
    client.post<ReportSessionResult>('/api/stats/reading-session', data),
  getGoal: () => client.get<ReadingGoalRecord>('/api/stats/reading-goal'),
  updateGoal: (daily_minutes: number, daily_chapters: number) =>
    client.put<ReadingGoalRecord>('/api/stats/reading-goal', { daily_minutes, daily_chapters }),
  today: () => client.get<TodayReading>('/api/stats/today'),
  history: (limit = 20) =>
    client.get<{ items: ReadingHistoryItem[] }>(`/api/stats/reading-history?limit=${limit}`),
}
