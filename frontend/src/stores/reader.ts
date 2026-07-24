import { defineStore } from 'pinia'
import { ref, watch } from 'vue'

export type ReaderTheme = 'paper' | 'sepia' | 'dark' | 'white'
export type PageMode = 'scroll' | 'paginate'

export interface ReaderSettings {
  fontSize: number      // 14-24
  lineHeight: number    // 1.5-2.5
  theme: ReaderTheme
  mode: PageMode
}

const READER_KEY = 'banzhu-reader-settings'
const DEFAULT_SETTINGS: ReaderSettings = {
  fontSize: 18,
  lineHeight: 1.8,
  theme: 'paper',
  mode: 'scroll',
}

function loadSettings(): ReaderSettings {
  try {
    const raw = localStorage.getItem(READER_KEY)
    if (!raw) return { ...DEFAULT_SETTINGS }
    const parsed = JSON.parse(raw) as Partial<ReaderSettings>
    return { ...DEFAULT_SETTINGS, ...parsed }
  } catch {
    return { ...DEFAULT_SETTINGS }
  }
}

export const useReaderStore = defineStore('reader', () => {
  const settings = ref<ReaderSettings>(loadSettings())

  function update(patch: Partial<ReaderSettings>) {
    settings.value = { ...settings.value, ...patch }
  }

  watch(settings, (val) => {
    localStorage.setItem(READER_KEY, JSON.stringify(val))
  }, { deep: true })

  return { settings, update }
})
