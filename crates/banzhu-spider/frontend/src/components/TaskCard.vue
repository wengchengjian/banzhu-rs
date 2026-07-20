<script setup lang="ts">
import { computed } from 'vue'
import { RouterLink } from 'vue-router'
import type { CrawlTask } from '@/api/crawl'
import { formatDate } from '@/utils/format'

const props = defineProps<{ task: CrawlTask }>()
const emit = defineEmits<{ retry: [bookId: number] }>()

// 状态 → 显示文本 + 颜色类
const STATUS_META: Record<string, { label: string; badge: string; bar: string }> = {
  pending: { label: '待处理', badge: 'bg-gray-100 text-gray-700 dark:bg-gray-700 dark:text-gray-300', bar: 'bg-gray-400' },
  running: { label: '运行中', badge: 'bg-blue-100 text-blue-700 dark:bg-blue-900 dark:text-blue-300', bar: 'bg-blue-500' },
  success: { label: '成功', badge: 'bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300', bar: 'bg-green-500' },
  failed: { label: '失败', badge: 'bg-red-100 text-red-700 dark:bg-red-900 dark:text-red-300', bar: 'bg-red-500' },
  skipped: { label: '已跳过', badge: 'bg-yellow-100 text-yellow-700 dark:bg-yellow-900 dark:text-yellow-300', bar: 'bg-yellow-500' },
}

const statusMeta = computed(() => STATUS_META[props.task.status] ?? STATUS_META.pending)

// 进度百分比：优先用 progress 字段；没有则用章节进度推算
const progressPercent = computed(() => {
  if (typeof props.task.progress === 'number' && props.task.progress > 0) {
    return Math.min(100, Math.max(0, props.task.progress))
  }
  if (props.task.chapters_total > 0) {
    return Math.min(100, Math.round((props.task.chapters_done / props.task.chapters_total) * 100))
  }
  return 0
})

const showProgress = computed(() =>
  props.task.status === 'running' || props.task.status === 'success' || props.task.progress > 0,
)

// 用于路由的 book_id：优先 book_id（书籍表主键），fallback 到 website_book_id
const routeBookId = computed(() => props.task.book_id ?? props.task.website_book_id)

// 显示用 ID：始终展示 website_book_id（来源网站 ID）
const displayBookId = computed(() => props.task.website_book_id)

const isFailed = computed(() => props.task.status === 'failed')

function onRetry() {
  emit('retry', props.task.website_book_id)
}
</script>

<template>
  <div
    class="rounded-lg border border-gray-200 bg-white p-4 transition hover:shadow-sm dark:border-gray-700 dark:bg-gray-800"
  >
    <!-- 头部：标题 + 状态徽章 -->
    <div class="flex items-start justify-between gap-3">
      <div class="min-w-0 flex-1">
        <RouterLink
          v-if="routeBookId"
          :to="`/book/${routeBookId}`"
          class="block truncate font-medium text-gray-900 hover:text-blue-600 hover:underline dark:text-gray-100"
          :title="task.title"
        >
          {{ task.title }}
        </RouterLink>
        <span
          v-else
          class="block truncate font-medium text-gray-900 dark:text-gray-100"
          :title="task.title"
        >
          {{ task.title }}
        </span>
      </div>
      <span
        class="flex-shrink-0 rounded-full px-2 py-0.5 text-xs font-medium"
        :class="statusMeta.badge"
      >
        {{ statusMeta.label }}
      </span>
    </div>

    <!-- 进度条 -->
    <div v-if="showProgress" class="mt-3">
      <div class="h-1.5 w-full overflow-hidden rounded-full bg-gray-100 dark:bg-gray-700">
        <div
          class="h-full transition-all"
          :class="statusMeta.bar"
          :style="{ width: `${progressPercent}%` }"
        />
      </div>
      <div class="mt-1 flex justify-between text-xs text-gray-500 dark:text-gray-400">
        <span>章节 {{ task.chapters_done }} / {{ task.chapters_total }}</span>
        <span>{{ progressPercent }}%</span>
      </div>
    </div>

    <!-- 错误消息 -->
    <p
      v-if="isFailed && task.error_message"
      class="mt-2 line-clamp-2 break-words rounded bg-red-50 px-2 py-1 text-xs text-red-600 dark:bg-red-900/30 dark:text-red-400"
    >
      {{ task.error_message }}
    </p>

    <!-- 底部：book_id + 时间 + 重试按钮 -->
    <div class="mt-3 flex items-center justify-between gap-2 text-xs text-gray-500 dark:text-gray-400">
      <div class="flex flex-wrap items-center gap-x-3 gap-y-1">
        <span>book_id: {{ displayBookId }}</span>
        <span v-if="task.started_at">开始: {{ formatDate(task.started_at) }}</span>
        <span v-if="task.finished_at">完成: {{ formatDate(task.finished_at) }}</span>
      </div>
      <button
        v-if="isFailed"
        type="button"
        class="flex-shrink-0 rounded border border-red-500 px-2 py-0.5 text-red-600 transition hover:bg-red-50 dark:text-red-400 dark:hover:bg-red-900/30"
        @click="onRetry"
      >
        重试
      </button>
    </div>
  </div>
</template>
