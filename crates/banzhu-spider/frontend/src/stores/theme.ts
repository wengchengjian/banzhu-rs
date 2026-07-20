import { defineStore } from 'pinia'
import { ref } from 'vue'

const THEME_KEY = 'banzhu-theme'
type Theme = 'light' | 'dark'

export const useThemeStore = defineStore('theme', () => {
  const current = ref<Theme>((localStorage.getItem(THEME_KEY) as Theme) || 'light')

  function apply(theme: Theme) {
    document.documentElement.classList.toggle('dark', theme === 'dark')
    localStorage.setItem(THEME_KEY, theme)
    current.value = theme
  }

  function toggle() {
    apply(current.value === 'dark' ? 'light' : 'dark')
  }

  apply(current.value)
  return { current, apply, toggle }
})
