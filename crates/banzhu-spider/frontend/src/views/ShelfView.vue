<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useShelfStore } from '@/stores/shelf'
import { useChapterCache } from '@/composables/useChapterCache'
import { useToast } from '@/composables/useToast'
import BookCard from '@/components/BookCard.vue'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import EmptyState from '@/components/EmptyState.vue'

const shelfStore = useShelfStore()
const cache = useChapterCache()
const toast = useToast()

const currentGroup = ref<string | undefined>(undefined)

const groups = [
  { label: '全部', value: undefined },
  { label: '在读', value: 'reading' },
  { label: '想读', value: 'want' },
  { label: '读完', value: 'finished' },
] as const

async function selectGroup(group: string | undefined) {
  currentGroup.value = group
  await shelfStore.load(group)
}

async function moveTo(bookId: number, group: string) {
  try {
    await shelfStore.move(bookId, group)
    toast.success('已移动分组')
  } catch (e) {
    toast.error((e as Error).message)
  }
}

async function removeFromShelf(bookId: number) {
  try {
    await shelfStore.remove(bookId)
    toast.success('已移出书架')
  } catch (e) {
    toast.error((e as Error).message)
  }
}

async function clearCache(bookId: number) {
  try {
    await cache.deleteBook(bookId)
    toast.success('已删除缓存')
  } catch (e) {
    toast.error((e as Error).message)
  }
}

onMounted(() => {
  shelfStore.load()
})
</script>

<template>
  <div class="container mx-auto px-4 py-6">
    <h1 class="mb-4 text-2xl font-bold text-gray-900 dark:text-gray-100">我的书架</h1>

    <!-- 标签页栏 -->
    <div class="mb-6 flex flex-wrap gap-2 border-b border-gray-200 dark:border-gray-700">
      <button
        v-for="g in groups"
        :key="g.label"
        class="rounded-t px-4 py-2 text-sm font-medium transition"
        :class="
          currentGroup === g.value
            ? 'border-b-2 border-blue-600 text-blue-600 dark:text-blue-400'
            : 'text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200'
        "
        @click="selectGroup(g.value)"
      >
        {{ g.label }}
      </button>
    </div>

    <!-- 加载中 -->
    <LoadingSpinner v-if="shelfStore.loading" />

    <!-- 错误 -->
    <EmptyState
      v-else-if="shelfStore.errorMsg"
      icon="⚠️"
      :message="`加载失败：${shelfStore.errorMsg}`"
    />

    <!-- 空书架 -->
    <EmptyState
      v-else-if="shelfStore.items.length === 0"
      icon="📚"
      message="书架空空如也"
    />

    <!-- 书籍列表 -->
    <div v-else class="space-y-4">
      <div
        v-for="item in shelfStore.items"
        :key="item.shelf.book_id"
        class="mb-4"
      >
        <BookCard :book="item.book" />
        <div class="mt-2 flex flex-wrap gap-2 pl-2">
          <button
            class="rounded border border-gray-300 px-2 py-1 text-xs text-gray-700 transition hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
            :class="
              item.shelf.group_name === 'reading'
                ? 'bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
                : ''
            "
            @click="moveTo(item.shelf.book_id, 'reading')"
          >
            在读
          </button>
          <button
            class="rounded border border-gray-300 px-2 py-1 text-xs text-gray-700 transition hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
            :class="
              item.shelf.group_name === 'want'
                ? 'bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
                : ''
            "
            @click="moveTo(item.shelf.book_id, 'want')"
          >
            想读
          </button>
          <button
            class="rounded border border-gray-300 px-2 py-1 text-xs text-gray-700 transition hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
            :class="
              item.shelf.group_name === 'finished'
                ? 'bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
                : ''
            "
            @click="moveTo(item.shelf.book_id, 'finished')"
          >
            读完
          </button>
          <button
            class="rounded border border-gray-300 px-2 py-1 text-xs text-gray-700 transition hover:bg-gray-100 dark:border-gray-600 dark:text-gray-300 dark:hover:bg-gray-700"
            @click="clearCache(item.shelf.book_id)"
          >
            删缓存
          </button>
          <button
            class="rounded border border-red-300 px-2 py-1 text-xs text-red-600 transition hover:bg-red-50 dark:border-red-700 dark:text-red-400 dark:hover:bg-red-900/30"
            @click="removeFromShelf(item.shelf.book_id)"
          >
            移出
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
