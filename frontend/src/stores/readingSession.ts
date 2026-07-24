import { defineStore } from 'pinia'
import { ref } from 'vue'
import { statsApi, type ReportSessionBody } from '@/api/stats'

const FLUSH_INTERVAL_SEC = 30  // 每 30 秒 flush 一次

export const useReadingSessionStore = defineStore('readingSession', () => {
  const bookId = ref(0)
  const chapterOrder = ref(0)
  const durationSec = ref(0)
  const chaptersRead = ref(0)
  const startedAt = ref(0)
  const timerId = ref<number | null>(null)
  const visible = ref(true)

  function onVisibility() {
    visible.value = !document.hidden
  }

  async function flush() {
    if (durationSec.value === 0 && chaptersRead.value === 0) return
    const body: ReportSessionBody = {
      book_id: bookId.value,
      chapter_order: chapterOrder.value,
      duration_sec: durationSec.value,
      chapters_read: chaptersRead.value,
      started_at: startedAt.value,
      ended_at: Math.floor(Date.now() / 1000),
    }
    // 立即重置状态，避免与 start 的新会话累加冲突
    durationSec.value = 0
    chaptersRead.value = 0
    startedAt.value = Math.floor(Date.now() / 1000)
    try {
      await statsApi.reportSession(body)
    } catch (e) {
      console.warn('上报阅读会话失败', e)
    }
  }

  function flushBeacon() {
    if (durationSec.value === 0 && chaptersRead.value === 0) return
    const body: ReportSessionBody = {
      book_id: bookId.value,
      chapter_order: chapterOrder.value,
      duration_sec: durationSec.value,
      chapters_read: chaptersRead.value,
      started_at: startedAt.value,
      ended_at: Math.floor(Date.now() / 1000),
    }
    const blob = new Blob([JSON.stringify(body)], { type: 'application/json' })
    navigator.sendBeacon('/api/stats/reading-session', blob)
    durationSec.value = 0
    chaptersRead.value = 0
    startedAt.value = Math.floor(Date.now() / 1000)
  }

  function start(bid: number, order: number) {
    // 切换书籍/章节时先 flush 旧会话
    if (bookId.value !== 0 && (bookId.value !== bid || chapterOrder.value !== order)) {
      flush()
    }
    bookId.value = bid
    chapterOrder.value = order
    startedAt.value = Math.floor(Date.now() / 1000)
    durationSec.value = 0
    chaptersRead.value = 0

    if (timerId.value !== null) clearInterval(timerId.value)
    timerId.value = window.setInterval(() => {
      if (visible.value) durationSec.value += 1
      if (durationSec.value > 0 && durationSec.value % FLUSH_INTERVAL_SEC === 0) {
        flush()
      }
    }, 1000)

    document.addEventListener('visibilitychange', onVisibility)
    window.addEventListener('beforeunload', flushBeacon)
  }

  function markChapterRead() {
    chaptersRead.value++
  }

  function stop() {
    flush()
    if (timerId.value !== null) {
      clearInterval(timerId.value)
      timerId.value = null
    }
    document.removeEventListener('visibilitychange', onVisibility)
    window.removeEventListener('beforeunload', flushBeacon)
  }

  return { start, stop, markChapterRead, flush }
})
