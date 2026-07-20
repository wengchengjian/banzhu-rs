<script setup lang="ts">
import { useToast } from '@/composables/useToast'
const { toasts, remove } = useToast()
const colorMap: Record<string, string> = {
  success: 'bg-green-500',
  error: 'bg-red-500',
  info: 'bg-blue-500',
  warning: 'bg-orange-500',
}
</script>
<template>
  <div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
    <TransitionGroup name="toast">
      <div
        v-for="t in toasts"
        :key="t.id"
        :class="['text-white px-4 py-2 rounded shadow-lg cursor-pointer', colorMap[t.type]]"
        @click="remove(t.id)"
      >
        {{ t.message }}
      </div>
    </TransitionGroup>
  </div>
</template>
<style scoped>
.toast-enter-active, .toast-leave-active { transition: all 0.3s; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translateX(20px); }
</style>
