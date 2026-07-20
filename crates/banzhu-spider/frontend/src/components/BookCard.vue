<script setup lang="ts">
import { useRouter } from 'vue-router'
import type { BookListItem } from '@/types/books'
import { formatWordCount, formatNumber } from '@/utils/format'

defineProps<{ book: BookListItem }>()

const router = useRouter()

function goToDetail(id: number) {
  router.push(`/book/${id}`)
}
</script>

<template>
  <div
    class="group cursor-pointer rounded-lg border border-gray-200 p-4 transition hover:shadow-md dark:border-gray-700 dark:bg-gray-800"
    @click="goToDetail(book.id)"
  >
    <div class="flex gap-3">
      <!-- 首字封面 -->
      <div
        class="flex h-16 w-12 flex-shrink-0 items-center justify-center rounded bg-gradient-to-br from-blue-500 to-purple-600 text-xl font-bold text-white"
      >
        {{ book.title.charAt(0) }}
      </div>
      <div class="min-w-0 flex-1">
        <h3 class="truncate font-medium text-gray-900 dark:text-gray-100">
          {{ book.title }}
        </h3>
        <p class="mt-1 text-sm text-gray-600 dark:text-gray-400">
          {{ book.author }}
        </p>
        <div class="mt-2 flex flex-wrap gap-2 text-xs text-gray-500 dark:text-gray-500">
          <span class="rounded bg-gray-100 px-2 py-0.5 dark:bg-gray-700">
            {{ book.category }}
          </span>
          <span>{{ formatWordCount(book.word_count) }}</span>
          <span>{{ book.chapter_count }}章</span>
          <span>❤ {{ formatNumber(book.likes) }}</span>
        </div>
      </div>
    </div>
  </div>
</template>
