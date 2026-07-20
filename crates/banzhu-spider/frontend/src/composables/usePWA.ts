import { ref } from 'vue'
import { registerSW } from 'virtual:pwa-register'

export function usePWA() {
  const needRefresh = ref(false)
  const offlineReady = ref(false)

  const updateSW = registerSW({
    onNeedRefresh() {
      needRefresh.value = true
    },
    onOfflineReady() {
      offlineReady.value = true
    },
  })

  function update() {
    updateSW(true)
  }

  function dismissOfflineReady() {
    offlineReady.value = false
  }

  return { needRefresh, offlineReady, update, dismissOfflineReady }
}
