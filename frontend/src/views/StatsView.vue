<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { RouterLink } from 'vue-router'
import {
  statsApi,
  type HeatmapPoint,
  type TimelinePoint,
  type TodayReading,
  type ReadingHistoryItem,
} from '@/api/stats'
import type { ReadingGoalRecord } from '@/types/api/ReadingGoalRecord'
import { useChapterCache } from '@/composables/useChapterCache'
import { useToast } from '@/composables/useToast'
import { useConfirm } from '@/composables/useConfirm'
import HeatmapCalendar from '@/components/HeatmapCalendar.vue'
import StatCard from '@/components/StatCard.vue'

const cache = useChapterCache()
const toast = useToast()
const { confirm } = useConfirm()

const todayData = ref<TodayReading>({ duration_sec: 0, chapters_read: 0 })
const goal = ref<ReadingGoalRecord | null>(null)
const heatmapData = ref<HeatmapPoint[]>([])
const timelineData = ref<TimelinePoint[]>([])
const history = ref<ReadingHistoryItem[]>([])
const cacheCounts = ref<Record<number, number>>({})

const selectedYear = ref(new Date().getFullYear())
const goalForm = ref({ daily_minutes: 0, daily_chapters: 0 })
const savingGoal = ref(false)
const loading = ref(true)
const errorMsg = ref('')

const todayMinutes = computed(() => Math.floor(todayData.value.duration_sec / 60))
const timelineMax = computed(() =>
  Math.max(...timelineData.value.map((d) => d.duration_sec), 1),
)

function formatDuration(sec: number): string {
  if (sec < 60) return `${sec}s`
  const m = Math.floor(sec / 60)
  if (m < 60) return `${m}m`
  return `${Math.floor(m / 60)}h ${m % 60}m`
}

function formatRelative(ts: number): string {
  const diff = Math.floor(Date.now() / 1000 - ts)
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

function formatDate(iso: string): string {
  return iso.slice(5) // MM-DD
}

function barHeight(duration: number): number {
  return Math.round((duration / timelineMax.value) * 90)
}

async function loadHeatmap() {
  try {
    const r = await statsApi.heatmap(selectedYear.value)
    heatmapData.value = r.items
  } catch (e) {
    toast.error(`加载热力图失败：${(e as Error).message}`)
  }
}

async function refreshCacheCounts() {
  for (const item of history.value) {
    const count = await cache.getBookCount(item.book_id).catch(() => 0)
    cacheCounts.value[item.book_id] = count
  }
}

async function onDeleteCache(bookId: number, title: string) {
  const ok = await confirm({
    title: '删除缓存',
    message: `确定删除《${title}》的章节缓存？`,
    confirmText: '删除',
  })
  if (!ok) return
  try {
    await cache.deleteBook(bookId)
    toast.success(`已删除《${title}》的缓存`)
    cacheCounts.value[bookId] = 0
  } catch {
    toast.error('删除失败')
  }
}

async function saveGoal() {
  savingGoal.value = true
  try {
    const updated = await statsApi.updateGoal(
      goalForm.value.daily_minutes,
      goalForm.value.daily_chapters,
    )
    goal.value = updated
    toast.success('目标已保存')
  } catch {
    toast.error('保存失败')
  } finally {
    savingGoal.value = false
  }
}

function changeYear(delta: number) {
  selectedYear.value += delta
  loadHeatmap()
}

onMounted(async () => {
  try {
    const [today, goalData, heatmap, timeline, historyData] = await Promise.all([
      statsApi.today(),
      statsApi.getGoal(),
      statsApi.heatmap(selectedYear.value),
      statsApi.timeline(7),
      statsApi.history(20),
    ])
    todayData.value = today
    goal.value = goalData
    heatmapData.value = heatmap.items
    timelineData.value = timeline.items
    history.value = historyData.items
    goalForm.value = {
      daily_minutes: goalData.daily_minutes,
      daily_chapters: goalData.daily_chapters,
    }
    await refreshCacheCounts()
  } catch (e) {
    errorMsg.value = (e as Error).message
  } finally {
    loading.value = false
  }
})
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <h1 class="mb-6 text-2xl font-bold text-gray-900 dark:text-gray-100">
      阅读统计
    </h1>

    <!-- 加载中 -->
    <p v-if="loading" class="text-gray-500 dark:text-gray-400">加载中...</p>

    <!-- 错误 -->
    <p v-else-if="errorMsg" class="text-red-600 dark:text-red-400">
      加载失败：{{ errorMsg }}
    </p>

    <div v-else class="space-y-6">
      <!-- Section 1: 今日进度 -->
      <section class="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <StatCard
          label="今日阅读"
          :value="todayMinutes"
          :total="goal?.daily_minutes"
          unit="分钟"
        />
        <StatCard
          label="今日章节"
          :value="todayData.chapters_read"
          :total="goal?.daily_chapters"
          unit="章"
        />
      </section>

      <!-- Section 2: 阅读热力图 -->
      <section
        class="rounded-lg border border-gray-200 bg-white p-5 dark:border-gray-700 dark:bg-gray-800"
      >
        <div class="mb-4 flex items-center justify-between">
          <h2 class="text-lg font-medium text-gray-900 dark:text-gray-100">
            阅读热力图
          </h2>
          <div class="flex items-center gap-2">
            <button
              type="button"
              class="rounded border border-gray-300 px-2 py-1 text-sm text-gray-700 transition hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
              @click="changeYear(-1)"
            >
              上一年
            </button>
            <span
              class="min-w-[3rem] text-center text-sm font-medium text-gray-900 dark:text-gray-100"
            >
              {{ selectedYear }}
            </span>
            <button
              type="button"
              class="rounded border border-gray-300 px-2 py-1 text-sm text-gray-700 transition hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
              @click="changeYear(1)"
            >
              下一年
            </button>
          </div>
        </div>
        <HeatmapCalendar :data="heatmapData" :year="selectedYear" />
      </section>

      <!-- Section 3: 最近 7 天明细 -->
      <section
        class="rounded-lg border border-gray-200 bg-white p-5 dark:border-gray-700 dark:bg-gray-800"
      >
        <h2 class="mb-4 text-lg font-medium text-gray-900 dark:text-gray-100">
          最近 7 天
        </h2>
        <svg
          :viewBox="`0 0 ${7 * 40} 100`"
          class="h-32 w-full"
          preserveAspectRatio="none"
        >
          <g v-for="(point, i) in timelineData" :key="point.date">
            <rect
              :x="i * 40"
              :y="100 - barHeight(point.duration_sec)"
              :width="30"
              :height="barHeight(point.duration_sec)"
              fill="#3b82f6"
            />
            <text
              :x="i * 40 + 15"
              y="98"
              text-anchor="middle"
              class="fill-current text-[10px] text-gray-500 dark:text-gray-400"
            >
              {{ formatDate(point.date) }}
            </text>
          </g>
        </svg>
      </section>

      <!-- Section 4: 阅读历史 + 缓存管理 -->
      <section
        class="rounded-lg border border-gray-200 bg-white p-5 dark:border-gray-700 dark:bg-gray-800"
      >
        <h2 class="mb-4 text-lg font-medium text-gray-900 dark:text-gray-100">
          阅读历史
        </h2>
        <ul class="divide-y divide-gray-100 dark:divide-gray-700">
          <li
            v-for="item in history"
            :key="item.book_id"
            class="py-3"
          >
            <div class="flex flex-wrap items-center justify-between gap-2">
              <div class="min-w-0 flex-1">
                <RouterLink
                  :to="`/book/${item.book_id}`"
                  class="truncate font-medium text-gray-900 hover:text-blue-600 hover:underline dark:text-gray-100"
                  :title="item.book_title"
                >
                  {{ item.book_title }}
                </RouterLink>
                <div
                  class="mt-1 flex flex-wrap gap-x-3 gap-y-1 text-xs text-gray-500 dark:text-gray-400"
                >
                  <span>时长 {{ formatDuration(item.total_duration_sec) }}</span>
                  <span>{{ item.total_chapters }} 章</span>
                  <span>{{ formatRelative(item.last_read_at) }}</span>
                  <span>已缓存 {{ cacheCounts[item.book_id] ?? 0 }} 章</span>
                </div>
              </div>
              <div class="flex flex-shrink-0 gap-2">
                <RouterLink
                  :to="`/read/${item.book_id}/${item.last_chapter_order || 1}`"
                  class="rounded border border-gray-300 px-2 py-1 text-xs text-gray-700 transition hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
                >
                  继续阅读
                </RouterLink>
                <button
                  v-if="(cacheCounts[item.book_id] ?? 0) > 0"
                  type="button"
                  class="rounded border border-red-500 px-2 py-1 text-xs text-red-600 transition hover:bg-red-50 dark:border-red-700 dark:text-red-400 dark:hover:bg-red-900/20"
                  @click="onDeleteCache(item.book_id, item.book_title)"
                >
                  删缓存
                </button>
              </div>
            </div>
          </li>
        </ul>
        <p
          v-if="history.length === 0"
          class="py-4 text-center text-sm text-gray-500 dark:text-gray-400"
        >
          暂无阅读历史
        </p>
      </section>

      <!-- Section 5: 设置阅读目标 -->
      <section
        class="rounded-lg border border-gray-200 bg-white p-5 dark:border-gray-700 dark:bg-gray-800"
      >
        <h2 class="mb-4 text-lg font-medium text-gray-900 dark:text-gray-100">
          阅读目标
        </h2>
        <form
          class="flex flex-wrap items-end gap-4"
          @submit.prevent="saveGoal"
        >
          <label class="flex flex-col gap-1 text-sm text-gray-700 dark:text-gray-300">
            每日分钟数
            <input
              v-model.number="goalForm.daily_minutes"
              type="number"
              min="0"
              class="w-28 rounded border border-gray-300 bg-white px-2 py-1 text-sm text-gray-900 focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
            />
          </label>
          <label class="flex flex-col gap-1 text-sm text-gray-700 dark:text-gray-300">
            每日章节数
            <input
              v-model.number="goalForm.daily_chapters"
              type="number"
              min="0"
              class="w-28 rounded border border-gray-300 bg-white px-2 py-1 text-sm text-gray-900 focus:border-blue-500 focus:outline-none dark:border-gray-600 dark:bg-gray-700 dark:text-gray-100"
            />
          </label>
          <button
            type="submit"
            class="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60"
            :disabled="savingGoal"
          >
            {{ savingGoal ? '保存中...' : '保存' }}
          </button>
        </form>
      </section>
    </div>
  </div>
</template>
