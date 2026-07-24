<script setup lang="ts">
defineProps<{ message?: string }>()
</script>
<template>
  <div class="flex items-center justify-center py-8 text-gray-400">
    <svg class="animate-spin h-5 w-5 mr-2" viewBox="0 0 24 24" fill="none">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.4 0 0 5.4 0 12h4z" />
    </svg>
    <span>{{ message ?? '加载中...' }}</span>
  </div>
</template>
