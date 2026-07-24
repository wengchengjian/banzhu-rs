import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import { useToast } from '@/composables/useToast'
import './assets/styles/main.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)

// 全局错误兜底
const toast = useToast()
app.config.errorHandler = (err, _instance, info) => {
  console.error('[Vue Error]', err, info)
  toast.error((err as Error)?.message ?? '应用发生未知错误')
}
window.addEventListener('unhandledrejection', (event) => {
  console.error('[Unhandled Promise]', event.reason)
  toast.error(event.reason?.message ?? '异步操作失败')
})

app.mount('#app')

// 申请持久化存储（PWA 离线数据不被浏览器自动清理）
if ('storage' in navigator && 'persist' in navigator.storage) {
  navigator.storage.persist().then((persisted) => {
    if (persisted) {
      console.log('[PWA] 持久化存储已启用')
    }
  }).catch(() => {
    // 静默失败，不影响应用启动
  })
}
