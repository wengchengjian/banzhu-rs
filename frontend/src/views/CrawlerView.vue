<script setup lang="ts">
import { ref, computed, onMounted, nextTick, watch } from 'vue'
import { useSSE } from '@/composables/useSSE'
import { useCrawlerStore } from '@/stores/crawler'
import { crawlApi } from '@/api/crawl'
import type { CrawlTask } from '@/api/crawl'
import { useToast } from '@/composables/useToast'
import StatCard from '@/components/StatCard.vue'
import TaskCard from '@/components/TaskCard.vue'
import EmptyState from '@/components/EmptyState.vue'

const store = useCrawlerStore()
const toast = useToast()

// ─── SSE ────────────────────────────────────────────────────────────────────
const { connected, error: sseError, connect, on } = useSSE({ url: '/api/crawl/stream' })

// ─── 手动爬取表单 ────────────────────────────────────────────────────────────
const manualUrl = ref('')
const submitting = ref(false)

async function submitManual() {
  const url = manualUrl.value.trim()
  if (!url) {
    toast.warning('请输入书籍 URL')
    return
  }
  submitting.value = true
  try {
    const result = await crawlApi.manual(url)
    toast.success(`已触发爬取，book_id = ${result.book_id}`)
    manualUrl.value = ''
    // 重新加载状态与任务列表
    await reloadStatusAndTasks()
  } catch (e) {
    toast.error(`爬取失败：${(e as Error).message}`)
  } finally {
    submitting.value = false
  }
}

// ─── 工具栏 ──────────────────────────────────────────────────────────────────
const searchQuery = ref('')
const retrying = ref(false)
const clearing = ref(false)

async function reloadTasks() {
  try {
    const res = await crawlApi.tasks({ page: 1, limit: 1000 })
    store.setTasks(res.items)
  } catch (e) {
    console.error('加载任务失败', e)
  }
}

async function reloadStatusAndTasks() {
  try {
    const [status, tasks] = await Promise.all([
      crawlApi.status(),
      crawlApi.tasks({ page: 1, limit: 1000 }),
    ])
    store.patchStatus(status)
    store.setTasks(tasks.items)
  } catch (e) {
    console.error('加载状态/任务失败', e)
  }
}

async function retryAllFailed() {
  retrying.value = true
  try {
    const result = await crawlApi.retryFailed()
    toast.success(`已重试 ${result.count} 个失败任务`)
    await reloadTasks()
  } catch (e) {
    toast.error(`重试失败：${(e as Error).message}`)
  } finally {
    retrying.value = false
  }
}

async function clearCompleted() {
  clearing.value = true
  try {
    const result = await crawlApi.deleteByStatus('success')
    toast.success(`已清除 ${result.count} 个已完成任务`)
    await reloadTasks()
  } catch (e) {
    toast.error(`清除失败：${(e as Error).message}`)
  } finally {
    clearing.value = false
  }
}

// ─── 分组折叠 ────────────────────────────────────────────────────────────────
const GROUP_LABELS: Record<string, string> = {
  failed: '失败',
  running: '运行中',
  pending: '待处理',
  success: '成功',
  skipped: '已跳过',
}

const collapsed = ref<Record<string, boolean>>({
  failed: false,   // 默认展开
  running: false,  // 默认展开
  pending: true,   // 默认折叠
  success: true,   // 默认折叠
  skipped: true,   // 默认折叠
})

function toggleGroup(group: string) {
  collapsed.value[group] = !collapsed.value[group]
}

const groupedTasks = computed(() => {
  const groups: Record<string, CrawlTask[]> = {
    failed: [],
    running: [],
    pending: [],
    success: [],
    skipped: [],
  }
  const q = searchQuery.value.trim().toLowerCase()
  for (const t of store.sortedTasks) {
    if (q && !t.title.toLowerCase().includes(q)) continue
    const s = t.status as keyof typeof groups
    if (s in groups) groups[s].push(t)
  }
  return groups
})

const totalTaskCount = computed(() =>
  Array.from(store.tasks.values()).length,
)

// ─── 顶部聚合卡片数据 ─────────────────────────────────────────────────────────
const status = computed(() => store.status)
const totalCount = computed(() => store.statusCount)

// ─── 日志面板 ─────────────────────────────────────────────────────────────────
const logPanelRef = ref<HTMLDivElement | null>(null)

const LOG_LEVEL_CLASS: Record<string, string> = {
  INFO: 'text-green-600 dark:text-green-400',
  WARN: 'text-yellow-600 dark:text-yellow-400',
  WARNING: 'text-yellow-600 dark:text-yellow-400',
  ERROR: 'text-red-600 dark:text-red-400',
}

function logLevelClass(level: string): string {
  return LOG_LEVEL_CLASS[level.toUpperCase()] ?? 'text-gray-600 dark:text-gray-400'
}

function formatLogTime(ts: number): string {
  const d = new Date(ts * 1000)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

async function scrollToBottom() {
  await nextTick()
  const el = logPanelRef.value
  if (el) el.scrollTop = el.scrollHeight
}

watch(() => store.logs.length, () => { scrollToBottom() })

// ─── 生命周期 ─────────────────────────────────────────────────────────────────
onMounted(async () => {
  // 1. 初始拉取 REST 数据
  try {
    const [statusData, tasks, logs] = await Promise.all([
      crawlApi.status(),
      crawlApi.tasks({ page: 1, limit: 1000 }),
      crawlApi.logs(200),
    ])
    store.patchStatus(statusData)
    store.setTasks(tasks.items)
    store.setLogs(logs)
  } catch (e) {
    console.error('初始加载失败', e)
    toast.error('初始加载失败，请刷新重试')
  }

  // 2. 注册 SSE 事件
  on('status', (data) => store.patchStatus(data as Parameters<typeof store.patchStatus>[0]))
  on('task:full', (data) => store.setTasks((data as { tasks: CrawlTask[] }).tasks))
  on('task:update', (data) => store.patchTask((data as { task: CrawlTask }).task))
  on('log', (data) => store.appendSSELog(data as Parameters<typeof store.appendSSELog>[0]))

  // 3. 连接 SSE
  connect()

  // 4. 日志面板初始滚到底部
  await scrollToBottom()
})

// useSSE 内部 onUnmounted 会自动 close，无需在此重复
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <h1 class="mb-6 text-2xl font-bold text-gray-900 dark:text-gray-100">爬虫控制台</h1>

    <!-- ─── 顶部聚合卡片 ─── -->
    <div class="mb-6 grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-5">
      <StatCard label="运行中" :value="totalCount.running" />
      <StatCard label="失败" :value="totalCount.failed" />
      <StatCard label="成功" :value="totalCount.success" />
      <StatCard label="待处理" :value="totalCount.pending" />
      <StatCard
        label="总进度"
        :value="status?.books_downloaded ?? 0"
        :total="status?.books_found ?? 0"
        unit="本"
      />
    </div>

    <!-- ─── 手动爬取表单 ─── -->
    <div class="mb-6 rounded-lg border border-gray-200 bg-white p-4 dark:border-gray-700 dark:bg-gray-800">
      <h2 class="mb-3 text-sm font-medium text-gray-700 dark:text-gray-300">手动爬取</h2>
      <div class="flex gap-2">
        <input
          v-model="manualUrl"
          type="url"
          placeholder="输入书籍详情页 URL，例如 https://www.example.com/book/123"
          class="flex-1 rounded border border-gray-300 px-3 py-2 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
          :disabled="submitting"
          @keyup.enter="submitManual"
        />
        <button
          type="button"
          class="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-700 disabled:opacity-50"
          :disabled="submitting"
          @click="submitManual"
        >
          {{ submitting ? '提交中...' : '开始爬取' }}
        </button>
      </div>
    </div>

    <!-- ─── 任务列表 ─── -->
    <div class="mb-6 rounded-lg border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800">
      <!-- 工具栏 -->
      <div class="flex flex-wrap items-center gap-2 border-b border-gray-200 p-4 dark:border-gray-700">
        <input
          v-model="searchQuery"
          type="text"
          placeholder="搜索任务标题..."
          class="flex-1 min-w-[200px] rounded border border-gray-300 px-3 py-1.5 text-sm outline-none focus:border-blue-500 focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-900 dark:text-gray-100"
        />
        <button
          type="button"
          class="rounded border border-orange-500 px-3 py-1.5 text-sm text-orange-600 transition hover:bg-orange-50 disabled:opacity-50 dark:text-orange-400 dark:hover:bg-orange-900/30"
          :disabled="retrying || totalCount.failed === 0"
          @click="retryAllFailed"
        >
          {{ retrying ? '重试中...' : `重试所有失败 (${totalCount.failed})` }}
        </button>
        <button
          type="button"
          class="rounded border border-gray-500 px-3 py-1.5 text-sm text-gray-600 transition hover:bg-gray-50 disabled:opacity-50 dark:text-gray-300 dark:hover:bg-gray-700"
          :disabled="clearing || totalCount.success === 0"
          @click="clearCompleted"
        >
          {{ clearing ? '清除中...' : '清空已完成' }}
        </button>
      </div>

      <!-- SSE 连接状态 -->
      <div class="flex items-center gap-2 border-b border-gray-200 px-4 py-2 text-xs dark:border-gray-700">
        <span
          class="inline-block h-2 w-2 rounded-full"
          :class="connected ? 'bg-green-500' : 'bg-gray-400'"
        />
        <span v-if="connected" class="text-green-600 dark:text-green-400">已连接</span>
        <span v-else-if="sseError" class="text-red-500 dark:text-red-400">
          未连接 · {{ sseError }}
        </span>
        <span v-else class="text-gray-500 dark:text-gray-400">未连接 · 重连中...</span>
      </div>

      <!-- 空状态 -->
      <EmptyState
        v-if="totalTaskCount === 0"
        icon="📋"
        message="暂无爬取任务，输入 URL 开始爬取吧"
      />

      <!-- 分组折叠列表 -->
      <template v-else>
        <div
          v-for="group in (['failed', 'running', 'pending', 'success', 'skipped'] as const)"
          :key="group"
        >
          <div
            v-if="groupedTasks[group].length > 0"
            class="border-b border-gray-200 last:border-b-0 dark:border-gray-700"
          >
            <!-- 分组标题 -->
            <button
              type="button"
              class="flex w-full items-center justify-between px-4 py-2 text-left text-sm font-medium text-gray-700 transition hover:bg-gray-50 dark:text-gray-300 dark:hover:bg-gray-700/50"
              @click="toggleGroup(group)"
            >
              <span>{{ GROUP_LABELS[group] }} ({{ groupedTasks[group].length }})</span>
              <span class="text-gray-400 transition-transform" :class="collapsed[group] ? '' : 'rotate-90'">▶</span>
            </button>
            <!-- 分组内容 -->
            <div
              v-show="!collapsed[group]"
              class="grid grid-cols-1 gap-3 p-4 pt-0 md:grid-cols-2 lg:grid-cols-3"
            >
              <TaskCard
                v-for="task in groupedTasks[group]"
                :key="task.website_book_id ?? task.id"
                :task="task"
                @retry="() => {}"
              />
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- ─── 日志面板 ─── -->
    <div class="rounded-lg border border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-800">
      <div class="flex items-center justify-between border-b border-gray-200 px-4 py-2 dark:border-gray-700">
        <h2 class="text-sm font-medium text-gray-700 dark:text-gray-300">实时日志</h2>
        <span class="text-xs text-gray-400">最近 {{ store.logs.length }} 条</span>
      </div>
      <div
        ref="logPanelRef"
        class="h-64 overflow-y-auto p-3 font-mono text-xs"
      >
        <div
          v-if="store.logs.length === 0"
          class="flex h-full items-center justify-center text-gray-400"
        >
          暂无日志
        </div>
        <div
          v-for="log in store.logs"
          :key="log.id"
          class="flex gap-2 py-0.5"
        >
          <span class="flex-shrink-0 text-gray-400">{{ formatLogTime(log.created_at) }}</span>
          <span class="flex-shrink-0 font-bold" :class="logLevelClass(log.level)">
            [{{ log.level.toUpperCase() }}]
          </span>
          <span class="break-all text-gray-700 dark:text-gray-300">{{ log.message }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
