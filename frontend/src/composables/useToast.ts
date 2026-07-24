import { ref } from 'vue'

export type ToastType = 'success' | 'error' | 'info' | 'warning'
export interface ToastItem {
  id: number
  type: ToastType
  message: string
}

const toasts = ref<ToastItem[]>([])
let nextId = 1

export function useToast() {
  function show(message: string, type: ToastType = 'info', duration = 3000) {
    const id = nextId++
    toasts.value.push({ id, type, message })
    setTimeout(() => remove(id), duration)
  }
  function remove(id: number) {
    toasts.value = toasts.value.filter(t => t.id !== id)
  }
  return {
    toasts,
    show,
    remove,
    success: (msg: string) => show(msg, 'success'),
    error: (msg: string) => show(msg, 'error', 5000),
    info: (msg: string) => show(msg, 'info'),
    warning: (msg: string) => show(msg, 'warning', 4000),
  }
}
