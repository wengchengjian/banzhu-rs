<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { booksApi } from '@/api/books'
import { shelfApi } from '@/api/shelf'
import { progressApi } from '@/api/progress'
import { useToast } from '@/composables/useToast'
import { useConfirm } from '@/composables/useConfirm'
import { useChapterCache } from '@/composables/useChapterCache'
import { formatWordCount, formatDate, formatNumber } from '@/utils/format'
import type { BookDetail, ChapterListItem } from '@/types/books'
import type { ReadingProgressRecord } from '@/types/api/ReadingProgressRecord'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import EmptyState from '@/components/EmptyState.vue'
import ChapterList from '@/components/ChapterList.vue'

const route = useRoute()
const router = useRouter()
const toast = useToast()
const { confirm } = useConfirm()
const cache = useChapterCache()

const book = ref<BookDetail | null>(null)
const chapters = ref<ChapterListItem[]>([])
const progress = ref<ReadingProgressRecord | null>(null)
const inShelf = ref(false)
const initialLoading = ref(true)
const errorMsg = ref('')
const cachedCount = ref(0)

const bookId = computed(() => Number(route.params.id))
const startOrder = computed(() => progress.value?.chapter_order ?? 1)

async function refreshCacheCount() {
  if (!bookId.value) return
  cachedCount.value = await cache.getBookCount(bookId.value).catch(() => 0)
}

async function deleteCache() {
  if (!book.value) return
  const ok = await confirm({
    title: '删除缓存',
    message: `确认删除《${book.value.title}》的 ${cachedCount.value} 章缓存？`,
    confirmText: '删除',
  })
  if (!ok) return
  try {
    // 1. 删除 IndexedDB 缓存
    await cache.deleteBook(bookId.value)
    // 2. 同时清理 SW Cache（chapters-cache）
    if ('caches' in window) {
      const swCache = await caches.open('chapters-cache')
      const keys = await swCache.keys()
      await Promise.all(
        keys
          .filter(req => req.url.includes(`/api/books/${bookId.value}/chapters/`))
          .map(req => swCache.delete(req)),
      )
      // 同时清理 books-cache（书籍详情缓存）
      const booksCache = await caches.open('books-cache')
      const bookKeys = await booksCache.keys()
      await Promise.all(
        bookKeys
          .filter(req => req.url.includes(`/api/books/${bookId.value}`))
          .filter(req => req.url.match(/\/api\/books\/\d+\/?$/))
          .map(req => booksCache.delete(req)),
      )
    }
    toast.success('缓存已删除')
    await refreshCacheCount()
  } catch {
    toast.error('删除失败')
  }
}

onMounted(async () => {
  try {
    const [bookRes, chaptersRes, progressRes, shelfRes] = await Promise.all([
      booksApi.get(bookId.value),
      booksApi.chapters(bookId.value),
      progressApi.get(bookId.value),
      shelfApi.list(),
    ])
    book.value = bookRes
    chapters.value = chaptersRes.items
    progress.value = progressRes
    inShelf.value = shelfRes.some(s => s.book_id === bookId.value)
  } catch (e) {
    errorMsg.value = (e as Error).message
  } finally {
    initialLoading.value = false
  }
  await refreshCacheCount()
})

function startReading() {
  router.push(`/read/${bookId.value}/${startOrder.value}`)
}

function goToChapter(order: number) {
  router.push(`/read/${bookId.value}/${order}`)
}

async function addToShelf() {
  if (inShelf.value) {
    toast.info('已在书架中')
    return
  }
  try {
    await shelfApi.add(bookId.value)
    inShelf.value = true
    toast.success('已加入书架')
  } catch (e) {
    toast.error(`加入书架失败：${(e as Error).message}`)
  }
}

function exportTxt() {
  booksApi.exportBook(bookId.value, 'txt')
}

function exportEpub() {
  booksApi.exportBook(bookId.value, 'epub')
}

async function deleteBook() {
  const ok = await confirm({
    title: '删除书籍',
    message: `确认删除《${book.value?.title}》吗？此操作不可恢复。`,
    confirmText: '删除',
  })
  if (!ok) return
  try {
    await booksApi.delete(bookId.value)
    toast.success('删除成功')
    router.push('/')
  } catch (e) {
    toast.error(`删除失败：${(e as Error).message}`)
  }
}
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <!-- 加载状态 -->
    <LoadingSpinner v-if="initialLoading" />

    <!-- 错误状态 -->
    <EmptyState
      v-else-if="errorMsg"
      icon="⚠️"
      :message="`加载失败：${errorMsg}`"
    />

    <!-- 详情内容 -->
    <template v-else-if="book">
      <!-- 书籍信息区 -->
      <div class="mb-6 flex gap-4">
        <!-- 首字封面 -->
        <div
          class="flex h-24 w-20 flex-shrink-0 items-center justify-center rounded-lg bg-gradient-to-br from-blue-500 to-purple-600 text-4xl font-bold text-white shadow-md"
        >
          {{ book.title.charAt(0) }}
        </div>
        <!-- 信息区 -->
        <div class="min-w-0 flex-1">
          <h1 class="text-2xl font-bold text-gray-900 dark:text-gray-100">
            {{ book.title }}
          </h1>
          <p class="mt-1 text-sm text-gray-600 dark:text-gray-400">
            作者：{{ book.author }}
          </p>
          <div class="mt-2 flex flex-wrap gap-2 text-xs text-gray-500 dark:text-gray-500">
            <span class="rounded bg-gray-100 px-2 py-0.5 dark:bg-gray-700">
              {{ book.category }}
            </span>
            <span>{{ formatWordCount(book.word_count) }}</span>
            <span>{{ book.status }}</span>
            <span>❤ {{ formatNumber(book.likes) }}</span>
            <span>收录：{{ formatDate(book.created_at) }}</span>
          </div>
        </div>
      </div>

      <!-- 简介 -->
      <div
        v-if="book.introduce"
        class="mb-6 whitespace-pre-wrap rounded-lg border border-gray-200 bg-gray-50 p-4 text-sm leading-relaxed text-gray-700 dark:border-gray-700 dark:bg-gray-800 dark:text-gray-300"
      >
        {{ book.introduce }}
      </div>

      <!-- 操作按钮区 -->
      <div class="mb-6 flex flex-wrap gap-2">
        <button
          class="rounded bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-700"
          @click="startReading"
        >
          开始阅读
        </button>
        <button
          class="rounded border border-gray-300 px-4 py-2 text-sm transition hover:bg-gray-100 dark:border-gray-600 dark:hover:bg-gray-800"
          :disabled="inShelf"
          :class="inShelf ? 'cursor-not-allowed opacity-60' : ''"
          @click="addToShelf"
        >
          {{ inShelf ? '已在书架' : '加入书架' }}
        </button>
        <button
          class="rounded border border-gray-300 px-4 py-2 text-sm transition hover:bg-gray-100 dark:border-gray-600 dark:hover:bg-gray-800"
          @click="exportTxt"
        >
          导出 TXT
        </button>
        <button
          class="rounded border border-gray-300 px-4 py-2 text-sm transition hover:bg-gray-100 dark:border-gray-600 dark:hover:bg-gray-800"
          @click="exportEpub"
        >
          导出 EPUB
        </button>
        <button
          class="rounded border border-red-500 px-4 py-2 text-sm text-red-600 transition hover:bg-red-50 dark:hover:bg-red-900/20"
          @click="deleteBook"
        >
          删除
        </button>
      </div>

      <!-- 章节列表区 -->
      <div>
        <div class="mb-3 flex items-center justify-between gap-2">
          <h2 class="text-lg font-medium text-gray-900 dark:text-gray-100">
            章节列表（共 {{ chapters.length }} 章）
          </h2>
          <div class="flex items-center gap-2 text-xs text-gray-500 dark:text-gray-400">
            <span>已缓存 {{ cachedCount }} / {{ chapters.length }} 章</span>
            <button
              v-if="cachedCount > 0"
              class="text-orange-500 hover:underline"
              @click="deleteCache"
            >
              删除缓存
            </button>
          </div>
        </div>
        <ChapterList
          :chapters="chapters"
          :current-order="progress?.chapter_order"
          @select="goToChapter"
        />
      </div>
    </template>
  </div>
</template>
