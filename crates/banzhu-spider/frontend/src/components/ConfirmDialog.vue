<script setup lang="ts">
import { useConfirm } from '@/composables/useConfirm'
const { visible, options, resolve } = useConfirm()
</script>
<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      @click.self="resolve(false)"
    >
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-sm w-full mx-4">
        <h3 class="text-lg font-medium mb-2">{{ options.title ?? '确认操作' }}</h3>
        <p class="text-gray-600 dark:text-gray-300 mb-4">{{ options.message }}</p>
        <div class="flex justify-end gap-2">
          <button class="px-4 py-1.5 text-sm border rounded" @click="resolve(false)">
            {{ options.cancelText ?? '取消' }}
          </button>
          <button class="px-4 py-1.5 text-sm bg-red-500 text-white rounded" @click="resolve(true)">
            {{ options.confirmText ?? '确认' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
