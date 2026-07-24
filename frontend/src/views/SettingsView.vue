<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useThemeStore } from '@/stores/theme'
import { useChapterCache } from '@/composables/useChapterCache'
import { useToast } from '@/composables/useToast'
import { useConfirm } from '@/composables/useConfirm'

const theme = useThemeStore()
const cache = useChapterCache()
const toast = useToast()
const { confirm } = useConfirm()

const usage = ref(0)
const quota = ref(0)
const clearing = ref(false)
const APP_VERSION = '0.1.0'

const percent = computed(() =>
  quota.value > 0 ? Math.min(100, (usage.value / quota.value) * 100) : 0,
)
const isWarning = computed(() => percent.value > 80)
const supported = computed(() => quota.value > 0)

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  return `${(bytes / 1024 / 1024 / 1024).toFixed(2)} GB`
}

async function loadSize() {
  const r = await cache.estimateSize()
  usage.value = r.usage
  quota.value = r.quota
}

async function handleClear() {
  const ok = await confirm({
    title: '清除缓存',
    message: '确认清除所有缓存？此操作不可恢复。',
    confirmText: '清除',
  })
  if (!ok) return
  clearing.value = true
  try {
    await cache.clearAll()
    toast.success('缓存已清除')
    await loadSize()
  } catch {
    toast.error('清除失败')
  } finally {
    clearing.value = false
  }
}

onMounted(loadSize)
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <h1 class="mb-6 text-2xl font-bold text-gray-900 dark:text-gray-100">设置</h1>

    <div class="max-w-2xl space-y-6">
      <!-- Section 1: 主题 -->
      <section
        class="rounded-lg border border-gray-200 bg-white p-5 dark:border-gray-700 dark:bg-gray-800"
      >
        <h2 class="mb-3 text-lg font-medium text-gray-900 dark:text-gray-100">主题</h2>
        <div class="flex gap-3">
          <button
            class="flex-1 rounded-lg border px-4 py-3 text-sm font-medium transition"
            :class="
              theme.current === 'light'
                ? 'border-blue-600 bg-blue-50 text-blue-700 ring-2 ring-blue-600/40 dark:bg-blue-900/30 dark:text-blue-300'
                : 'border-gray-300 text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700'
            "
            @click="theme.apply('light')"
          >
            浅色
          </button>
          <button
            class="flex-1 rounded-lg border px-4 py-3 text-sm font-medium transition"
            :class="
              theme.current === 'dark'
                ? 'border-blue-600 bg-blue-50 text-blue-700 ring-2 ring-blue-600/40 dark:bg-blue-900/30 dark:text-blue-300'
                : 'border-gray-300 text-gray-700 hover:bg-gray-50 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700'
            "
            @click="theme.apply('dark')"
          >
            暗黑
          </button>
        </div>
      </section>

      <!-- Section 2: 离线缓存 -->
      <section
        class="rounded-lg border border-gray-200 bg-white p-5 dark:border-gray-700 dark:bg-gray-800"
      >
        <h2 class="mb-3 text-lg font-medium text-gray-900 dark:text-gray-100">离线缓存</h2>

        <!-- 浏览器不支持存储估算 -->
        <p
          v-if="!supported"
          class="mb-4 text-sm text-gray-500 dark:text-gray-400"
        >
          浏览器不支持存储估算
        </p>

        <!-- 进度条 -->
        <div v-else class="mb-4">
          <div
            class="mb-2 flex justify-between text-sm text-gray-600 dark:text-gray-400"
          >
            <span>{{ formatBytes(usage) }} / {{ formatBytes(quota) }}</span>
            <span :class="isWarning ? 'font-medium text-orange-500' : ''">
              {{ percent.toFixed(1) }}%
            </span>
          </div>
          <div
            class="h-2 w-full overflow-hidden rounded-full bg-gray-200 dark:bg-gray-700"
          >
            <div
              class="h-full rounded-full transition-all duration-300"
              :class="isWarning ? 'bg-orange-500' : 'bg-blue-500'"
              :style="{ width: `${percent}%` }"
            ></div>
          </div>
        </div>

        <button
          class="rounded border border-red-500 px-4 py-2 text-sm text-red-600 transition hover:bg-red-50 disabled:cursor-not-allowed disabled:opacity-60 dark:border-red-700 dark:text-red-400 dark:hover:bg-red-900/20"
          :disabled="clearing"
          @click="handleClear"
        >
          {{ clearing ? '清除中...' : '清除全部缓存' }}
        </button>
      </section>

      <!-- Section 3: 关于 -->
      <section
        class="rounded-lg border border-gray-200 bg-white p-5 dark:border-gray-700 dark:bg-gray-800"
      >
        <h2 class="mb-3 text-lg font-medium text-gray-900 dark:text-gray-100">关于</h2>
        <dl class="text-sm">
          <div
            class="flex justify-between border-b border-gray-100 py-2 dark:border-gray-700"
          >
            <dt class="text-gray-600 dark:text-gray-400">版本</dt>
            <dd class="font-mono text-gray-900 dark:text-gray-100">
              v{{ APP_VERSION }}
            </dd>
          </div>
        </dl>
      </section>
    </div>
  </div>
</template>