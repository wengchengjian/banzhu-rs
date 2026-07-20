<script setup lang="ts">
import AppHeader from '@/components/AppHeader.vue'
import ToastContainer from '@/components/ToastContainer.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
</script>
<template>
  <AppHeader />
  <RouterView />
  <ToastContainer />
  <ConfirmDialog />
</template>
