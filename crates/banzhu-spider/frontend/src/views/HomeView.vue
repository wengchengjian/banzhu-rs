<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { booksApi } from '@/api/books'
import { categoriesApi } from '@/api/categories'
import type { BookListItem } from '@/types/books'
import BookCard from '@/components/BookCard.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import EmptyState from '@/components/EmptyState.vue'
import { useInfiniteScroll } from '@/composables/useInfiniteScroll'

const books = ref<BookListItem[]>([])
const categories = ref<string[]>([])
const selectedCategory = ref<string>('')
const page = ref(1)
const pageSize = 20
const initialLoading = ref(true)
const errorMsg = ref('')

const { sentinel, loading, hasMore, reset, check } = useInfiniteScroll({
  loadMore: async () => {
    try {
      const result = await booksApi.list({
        page: page.value,
        limit: pageSize,
        category: selectedCategory.value || undefined,
      })
      if (result.items.length === 0) {
        return false
      }
      books.value.push(...result.items)
      page.value += 1
      // 如果已加载完全部，标记没有更多
      if (books.value.length >= result.total) {
        return false
      }
    } catch (e) {
      errorMsg.value = (e as Error).message
      return false
    }
  },
})

async function loadCategories() {
  try {
    const result = await categoriesApi.list()
    categories.value = result.categories
  } catch (e) {
    // 分类加载失败不阻塞主流程
    console.error('Failed to load categories:', e)
  }
}

function selectCategory(cat: string) {
  if (selectedCategory.value === cat) return
  selectedCategory.value = cat
  books.value = []
  page.value = 1
  errorMsg.value = ''
  reset()
  // 手动触发首屏加载（不依赖 IntersectionObserver）
  check()
}

async function loadFirstPage() {
  try {
    const result = await booksApi.list({
      page: page.value,
      limit: pageSize,
      category: selectedCategory.value || undefined,
    })
    books.value = result.items
    page.value = 2 // 下一页
    if (books.value.length >= result.total) {
      hasMore.value = false
    }
  } catch (e) {
    errorMsg.value = (e as Error).message
  }
}

onMounted(async () => {
  // 并行加载分类和第一页书籍
  await Promise.all([loadCategories(), loadFirstPage()])
  initialLoading.value = false
})
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <!-- 分类筛选栏 -->
    <div
      v-if="categories.length > 0"
      class="mb-6 flex gap-2 overflow-x-auto pb-2"
    >
      <button
        class="flex-shrink-0 rounded-full px-4 py-1.5 text-sm transition"
        :class="
          selectedCategory === ''
            ? 'bg-blue-600 text-white'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700'
        "
        @click="selectCategory('')"
      >
        全部
      </button>
      <button
        v-for="cat in categories"
        :key="cat"
        class="flex-shrink-0 rounded-full px-4 py-1.5 text-sm transition"
        :class="
          selectedCategory === cat
            ? 'bg-blue-600 text-white'
            : 'bg-gray-100 text-gray-700 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700'
        "
        @click="selectCategory(cat)"
      >
        {{ cat }}
      </button>
    </div>

    <!-- 初始加载 -->
    <LoadingSpinner v-if="initialLoading" />

    <!-- 错误 -->
    <EmptyState
      v-else-if="errorMsg"
      icon="⚠️"
      :message="`加载失败：${errorMsg}`"
    />

    <!-- 空状态 -->
    <EmptyState
      v-else-if="books.length === 0 && !loading"
      icon="📚"
      message="暂无书籍，试试切换分类或稍后再来"
    />

    <!-- 书籍网格 -->
    <template v-else>
      <div class="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
        <BookCard v-for="book in books" :key="book.id" :book="book" />
      </div>

      <!-- 无限滚动 sentinel -->
      <div ref="sentinel" class="h-4" />

      <!-- 加载更多指示器 -->
      <div v-if="loading" class="py-4 text-center text-gray-500">
        <LoadingSpinner />
      </div>

      <!-- 没有更多 -->
      <div
        v-else-if="!hasMore && books.length > 0"
        class="py-4 text-center text-sm text-gray-400"
      >
        没有更多了
      </div>
    </template>
  </div>
</template>
