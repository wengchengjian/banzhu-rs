<script setup lang="ts">
import { RouterLink } from 'vue-router'
</script>
<template>
  <div class="container mx-auto p-4">
    <h1 class="text-2xl">404 — 页面未找到</h1>
    <p class="mt-2">
      <RouterLink to="/" class="text-blue-600 hover:underline dark:text-blue-400">
        返回首页
      </RouterLink>
    </p>
  </div>
</template>
