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
