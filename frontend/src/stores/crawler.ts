import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type { CrawlStatus, CrawlTask, CrawlLog, SSELogEvent } from '@/api/crawl'

/**
 * 爬虫状态 store。
 *
 * 数据来源有三处：
 * 1. REST `/api/crawl/status`、`/api/crawl/tasks`、`/api/crawl/logs` 初始拉取
 * 2. SSE `status` / `task:full` / `task:update` / `log` 事件实时推送
 * 3. 本地手动操作（manual / retryFailed / deleteByStatus）后刷新
 */
export const useCrawlerStore = defineStore('crawler', () => {
  const status = ref<CrawlStatus | null>(null)
  const tasks = ref<Map<number, CrawlTask>>(new Map())
  const logs = ref<CrawlLog[]>([])

  /** 任务状态计数（基于当前 tasks 计算，与后端 status_count 字段含义一致） */
  const statusCount = computed(() => {
    const counts = { pending: 0, running: 0, success: 0, failed: 0, skipped: 0 }
    for (const t of tasks.value.values()) {
      const s = t.status as keyof typeof counts
      if (s in counts) counts[s]++
    }
    return counts
  })

  /** 按状态优先级排序的任务列表：failed > running > pending > success > skipped */
  const sortedTasks = computed(() => {
    const order: Record<string, number> = {
      failed: 0,
      running: 1,
      pending: 2,
      success: 3,
      skipped: 4,
    }
    return Array.from(tasks.value.values()).sort((a, b) => {
      const oa = order[a.status] ?? 99
      const ob = order[b.status] ?? 99
      return oa - ob
    })
  })

  function patchStatus(s: CrawlStatus) {
    // 合并而非覆盖：SSE status 事件不携带 last_run，避免丢失 REST 拉取的 last_run
    status.value = { ...status.value, ...s }
  }

  function setTasks(items: CrawlTask[]) {
    tasks.value = new Map(items.map(t => [t.website_book_id ?? t.id, t]))
  }

  function patchTask(task: CrawlTask) {
    const key = task.website_book_id ?? task.id
    tasks.value.set(key, task)
    // 重新赋值 Map 以触发响应式
    tasks.value = new Map(tasks.value)
  }

  function appendLog(log: CrawlLog) {
    logs.value.push(log)
    if (logs.value.length > 200) logs.value = logs.value.slice(-200)
  }

  /** 适配 SSE `log` 事件（字段名是 timestamp，不是 created_at） */
  function appendSSELog(log: SSELogEvent) {
    appendLog({
      id: log.id,
      level: log.level,
      message: log.message,
      created_at: log.timestamp,
    })
  }

  function setLogs(items: CrawlLog[]) {
    logs.value = items.slice(-200)
  }

  function clearTasks() {
    tasks.value = new Map()
  }

  return {
    status,
    tasks,
    logs,
    statusCount,
    sortedTasks,
    patchStatus,
    setTasks,
    patchTask,
    appendLog,
    appendSSELog,
    setLogs,
    clearTasks,
  }
})
