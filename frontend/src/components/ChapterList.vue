<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import type { ChapterListItem } from '@/types/books'

const props = withDefaults(
  defineProps<{
    chapters: ChapterListItem[]
    currentOrder?: number
    pageSize?: number
  }>(),
  {
    pageSize: 100,
  },
)

const emit = defineEmits<{ select: [order: number] }>()

const currentPage = ref(1)

const totalPages = computed(() =>
  Math.max(1, Math.ceil(props.chapters.length / props.pageSize)),
)

const pagedChapters = computed(() => {
  const start = (currentPage.value - 1) * props.pageSize
  const end = currentPage.value * props.pageSize
  return props.chapters.slice(start, end)
})

function goToPage(page: number) {
  if (page < 1 || page > totalPages.value) return
  currentPage.value = page
}

function prevPage() {
  goToPage(currentPage.value - 1)
}

function nextPage() {
  goToPage(currentPage.value + 1)
}

function onSelect(chapter: ChapterListItem) {
  emit('select', chapter.order)
}

// chapters 变化时重置到第一页
watch(
  () => props.chapters,
  () => {
    currentPage.value = 1
  },
)
</script>

<template>
  <div>
    <!-- 空状态 -->
    <div
      v-if="chapters.length === 0"
      class="py-8 text-center text-sm text-gray-500 dark:text-gray-400"
    >
      暂无章节
    </div>

    <template v-else>
      <!-- 章节列表 -->
      <ul class="divide-y divide-gray-100 dark:divide-gray-700">
        <li
          v-for="chapter in pagedChapters"
          :key="chapter.id"
          class="cursor-pointer px-3 py-2.5 text-sm transition hover:bg-gray-50 dark:hover:bg-gray-800"
          :class="
            chapter.order === currentOrder
              ? 'bg-blue-50 text-blue-600 dark:bg-blue-900/30 dark:text-blue-400'
              : 'text-gray-700 dark:text-gray-300'
          "
          @click="onSelect(chapter)"
        >
          <span class="mr-2 text-xs text-gray-400">第{{ chapter.order }}章</span>
          {{ chapter.title }}
        </li>
      </ul>

      <!-- 分页控件 -->
      <div
        v-if="totalPages > 1"
        class="mt-4 flex items-center justify-center gap-3 text-sm text-gray-600 dark:text-gray-400"
      >
        <button
          class="rounded border border-gray-300 px-3 py-1 transition disabled:cursor-not-allowed disabled:opacity-40 dark:border-gray-600"
          :disabled="currentPage <= 1"
          @click="prevPage"
        >
          上一页
        </button>
        <span class="min-w-[80px] text-center">
          {{ currentPage }} / {{ totalPages }}
        </span>
        <button
          class="rounded border border-gray-300 px-3 py-1 transition disabled:cursor-not-allowed disabled:opacity-40 dark:border-gray-600"
          :disabled="currentPage >= totalPages"
          @click="nextPage"
        >
          下一页
        </button>
      </div>
    </template>
  </div>
</template>
