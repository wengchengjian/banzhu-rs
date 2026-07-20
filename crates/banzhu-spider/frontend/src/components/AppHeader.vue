<script setup lang="ts">
import { ref } from 'vue'
import { RouterLink, useRouter } from 'vue-router'
import { useThemeStore } from '@/stores/theme'

const router = useRouter()
const themeStore = useThemeStore()

const query = ref('')

function onSearch() {
  const q = query.value.trim()
  if (!q) return
  router.push('/search?q=' + encodeURIComponent(q))
}
</script>

<template>
  <header
    class="sticky top-0 z-10 border-b border-gray-200 bg-white dark:border-gray-700 dark:bg-gray-900"
  >
    <div class="container mx-auto flex items-center gap-4 px-4 py-3">
      <RouterLink
        to="/"
        class="text-lg font-bold text-gray-900 dark:text-gray-100"
      >
        banzhu-rs
      </RouterLink>

      <nav class="hidden items-center gap-4 md:flex">
        <RouterLink
          to="/"
          class="text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
          active-class="text-blue-600 dark:text-blue-400 font-medium"
        >
          首页
        </RouterLink>
        <RouterLink
          to="/shelf"
          class="text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
          active-class="text-blue-600 dark:text-blue-400 font-medium"
        >
          书架
        </RouterLink>
        <RouterLink
          to="/crawler"
          class="text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
          active-class="text-blue-600 dark:text-blue-400 font-medium"
        >
          爬虫
        </RouterLink>
        <RouterLink
          to="/stats"
          class="text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
          active-class="text-blue-600 dark:text-blue-400 font-medium"
        >
          统计
        </RouterLink>
        <RouterLink
          to="/settings"
          class="text-gray-700 hover:text-gray-900 dark:text-gray-300 dark:hover:text-gray-100"
          active-class="text-blue-600 dark:text-blue-400 font-medium"
        >
          设置
        </RouterLink>
      </nav>

      <div class="ml-auto flex items-center gap-2">
        <input
          v-model="query"
          type="text"
          placeholder="搜索..."
          class="w-40 rounded border border-gray-300 bg-white px-3 py-1.5 text-sm text-gray-900 focus:outline-none focus:ring-1 focus:ring-blue-500 dark:border-gray-600 dark:bg-gray-800 dark:text-gray-100"
          @keyup.enter="onSearch"
        />
        <button
          type="button"
          class="rounded px-2 py-1 text-gray-700 hover:bg-gray-100 dark:text-gray-300 dark:hover:bg-gray-800"
          :aria-label="themeStore.current === 'dark' ? '切换到浅色模式' : '切换到深色模式'"
          @click="themeStore.toggle()"
        >
          {{ themeStore.current === 'dark' ? '☀️' : '🌙' }}
        </button>
      </div>
    </div>
  </header>
</template>
