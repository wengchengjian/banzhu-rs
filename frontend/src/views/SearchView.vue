<script setup lang="ts">
import { ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { searchApi } from '@/api/search'
import type { SearchResult, SearchField } from '@/api/search'
import { formatWordCount } from '@/utils/format'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import EmptyState from '@/components/EmptyState.vue'

const route = useRoute()
const router = useRouter()

const query = ref((route.query.q as string) ?? '')
const field = ref<SearchField>((route.query.field as SearchField) ?? 'all')
const results = ref<SearchResult[]>([])
const total = ref(0)
const loading = ref(false)
const initialLoading = ref(false)
const errorMsg = ref('')
const hasSearched = ref(false)

async function doSearch() {
  const q = query.value.trim()
  if (!q) return
  // 首次搜索（无任何结果）用 initialLoading 显示更醒目的 loading
  if (!hasSearched.value) initialLoading.value = true
  loading.value = true
  errorMsg.value = ''
  try {
    const res = await searchApi.search({
      q,
      field: field.value,
      page: 1,
      limit: 20,
    })
    results.value = res.items
    total.value = res.total
    hasSearched.value = true
  } catch (e) {
    errorMsg.value = (e as Error).message
    results.value = []
    total.value = 0
  } finally {
    loading.value = false
    initialLoading.value = false
  }
}

function onSubmit() {
  const q = query.value.trim()
  if (!q) return
  router.replace({
    query: {
      q,
      field: field.value !== 'all' ? field.value : undefined,
    },
  })
}

/// 转义 HTML 特殊字符后再把 >>>...<<< 转为 <mark>，防止 XSS
function highlightSnippet(snippet: string): string {
  const escaped = snippet
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
  return escaped.replace(/&gt;&gt;&gt;([\s\S]*?)&lt;&lt;&lt;/g, '<mark>$1</mark>')
}

function goToDetail(bookId: number) {
  router.push(`/book/${bookId}`)
}

// 监听 route.query 变化自动搜索（首次进入 / 切换链接都覆盖）
watch(
  () => [route.query.q, route.query.field] as const,
  ([newQ, newField]) => {
    if (!newQ) {
      // 无 query 时重置为初始状态
      query.value = ''
      field.value = 'all'
      results.value = []
      total.value = 0
      hasSearched.value = false
      errorMsg.value = ''
      return
    }
    query.value = newQ as string
    field.value = (newField as SearchField) ?? 'all'
    doSearch()
  },
  { immediate: true },
)
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <!-- 搜索表单 -->
    <form
      class="mb-6 flex flex-wrap gap-2"
      @submit.prevent="onSubmit"
    >
      <input
        v-model="query"
        type="text"
        placeholder="输入关键词搜索书籍..."
        class="min-w-0 flex-1 rounded border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
      />
      <select
        v-model="field"
        class="rounded border border-gray-300 bg-white px-3 py-2 text-sm text-gray-900 focus:outline-none focus:ring-2 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
      >
        <option value="all">全部</option>
        <option value="title">标题</option>
        <option value="author">作者</option>
        <option value="content">内容</option>
      </select>
      <button
        type="submit"
        class="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-700 disabled:cursor-not-allowed disabled:opacity-60"
        :disabled="!query.trim() || loading"
      >
        搜索
      </button>
    </form>

    <!-- 初始加载 -->
    <LoadingSpinner v-if="initialLoading" />

    <!-- 错误状态 -->
    <EmptyState
      v-else-if="errorMsg"
      icon="⚠️"
      :message="`搜索失败：${errorMsg}`"
    />

    <!-- 空结果 -->
    <EmptyState
      v-else-if="hasSearched && results.length === 0 && !loading"
      icon="🔍"
      message="未找到相关书籍"
    />

    <!-- 结果列表 -->
    <template v-else>
      <div
        v-if="hasSearched && total > 0"
        class="mb-3 text-sm text-gray-500 dark:text-gray-400"
      >
        共找到 {{ total }} 条结果
      </div>

      <div class="space-y-3">
        <div
          v-for="r in results"
          :key="r.book_id"
          class="group cursor-pointer rounded-lg border border-gray-200 p-4 transition hover:shadow-md dark:border-gray-700 dark:bg-gray-800"
          @click="goToDetail(r.book_id)"
        >
          <div class="flex gap-3">
            <!-- 首字封面 -->
            <div
              class="flex h-16 w-12 flex-shrink-0 items-center justify-center rounded bg-gradient-to-br from-blue-500 to-purple-600 text-xl font-bold text-white"
            >
              {{ r.title.charAt(0) }}
            </div>
            <div class="min-w-0 flex-1">
              <h3 class="truncate font-medium text-gray-900 dark:text-gray-100">
                {{ r.title }}
              </h3>
              <p class="mt-1 text-sm text-gray-600 dark:text-gray-400">
                {{ r.author }}
              </p>
              <div class="mt-2 flex flex-wrap gap-2 text-xs text-gray-500 dark:text-gray-500">
                <span class="rounded bg-gray-100 px-2 py-0.5 dark:bg-gray-700">
                  {{ r.category }}
                </span>
                <span>{{ formatWordCount(r.word_count) }}</span>
                <span title="相关度">★ {{ r.relevance_score.toFixed(2) }}</span>
              </div>
            </div>
          </div>
          <!-- snippet 高亮 -->
          <p
            v-if="r.snippet"
            class="snippet mt-2 line-clamp-3 text-sm leading-relaxed text-gray-700 dark:text-gray-300"
            v-html="highlightSnippet(r.snippet)"
          />
        </div>
      </div>

      <!-- 加载更多指示器（搜索中） -->
      <div v-if="loading" class="py-4">
        <LoadingSpinner />
      </div>
    </template>
  </div>
</template>

<style scoped>
.snippet :deep(mark) {
  background-color: #fef08a;
  color: #92400e;
  border-radius: 2px;
  padding: 0 2px;
}
html.dark .snippet :deep(mark) {
  background-color: #ca8a04;
  color: #fff;
}
</style>
