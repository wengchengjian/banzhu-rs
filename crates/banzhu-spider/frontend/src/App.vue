<script setup lang="ts">
import AppHeader from '@/components/AppHeader.vue'
import ToastContainer from '@/components/ToastContainer.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
import { usePWA } from '@/composables/usePWA'

const { needRefresh, offlineReady, update, dismissOfflineReady } = usePWA()
</script>

<template>
  <AppHeader />
  <RouterView />
  <ToastContainer />
  <ConfirmDialog />
  <!-- PWA 更新提示 -->
  <div
    v-if="needRefresh"
    class="fixed bottom-4 right-4 z-50 rounded-lg bg-blue-500 px-4 py-3 text-white shadow-lg"
  >
    <span class="text-sm">发现新版本，</span>
    <button class="font-medium underline" @click="update">点击刷新</button>
  </div>
  <!-- PWA 离线就绪提示（仅显示一次） -->
  <div
    v-if="offlineReady"
    class="fixed bottom-4 right-4 z-50 rounded-lg bg-gray-700 px-4 py-3 text-white shadow-lg"
  >
    <span class="text-sm">应用已可离线使用</span>
    <button class="ml-2 underline" @click="dismissOfflineReady">关闭</button>
  </div>
</template>
