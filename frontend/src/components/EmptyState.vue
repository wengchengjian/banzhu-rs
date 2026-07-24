<script setup lang="ts">
import { computed, useAttrs } from 'vue'
defineProps<{ message?: string; icon?: string }>()
defineEmits<{ retry: [] }>()
const attrs = useAttrs()
const showRetry = computed(() => 'onRetry' in attrs)
</script>
<template>
  <div class="flex flex-col items-center justify-center py-12 text-gray-500">
    <div class="text-4xl mb-2">{{ icon ?? '📭' }}</div>
    <p>{{ message ?? '暂无数据' }}</p>
    <button
      v-if="showRetry"
      class="mt-3 px-4 py-1 text-sm text-blue-500 border border-blue-500 rounded hover:bg-blue-50"
      @click="$emit('retry')"
    >
      重试
    </button>
  </div>
</template>
