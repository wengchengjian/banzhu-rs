<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{
  label: string
  value: number | string
  total?: number
  unit?: string
}>()
const percent = computed(() => {
  if (props.total === undefined || !props.total) return 0
  return Math.min(100, Math.round((Number(props.value) / props.total) * 100))
})
</script>
<template>
  <div class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
    <div class="text-sm text-gray-500 dark:text-gray-400">{{ label }}</div>
    <div class="mt-1 flex items-baseline">
      <span class="text-2xl font-bold">{{ value }}</span>
      <span v-if="total" class="text-sm text-gray-400 ml-1">/ {{ total }}</span>
      <span v-if="unit" class="text-sm text-gray-400 ml-1">{{ unit }}</span>
    </div>
    <div v-if="total !== undefined" class="mt-2 h-1.5 bg-gray-100 rounded-full overflow-hidden">
      <div class="h-full bg-blue-500 transition-all" :style="{ width: `${percent}%` }" />
    </div>
  </div>
</template>
