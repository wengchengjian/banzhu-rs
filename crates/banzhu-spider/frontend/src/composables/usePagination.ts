import { computed, ref, watch } from 'vue'

const CHARS_PER_PAGE = 800
const MOBILE_BREAKPOINT = 768

export function usePagination(
  content: () => string,
  containerWidth: () => number,
) {
  const currentPage = ref(0)

  const isPaginated = computed(() => containerWidth() < MOBILE_BREAKPOINT)

  const pages = computed<string[]>(() => {
    const text = content()
    if (!isPaginated.value) return [text]
    const total = Math.max(1, Math.ceil(text.length / CHARS_PER_PAGE))
    const result: string[] = []
    for (let i = 0; i < total; i++) {
      result.push(text.slice(i * CHARS_PER_PAGE, (i + 1) * CHARS_PER_PAGE))
    }
    return result
  })

  const totalPages = computed(() => pages.value.length)
  const currentContent = computed(() => pages.value[currentPage.value] ?? '')

  function next(): boolean {
    if (currentPage.value < totalPages.value - 1) {
      currentPage.value++
      return true
    }
    return false
  }

  function prev(): boolean {
    if (currentPage.value > 0) {
      currentPage.value--
      return true
    }
    return false
  }

  function goTo(page: number) {
    currentPage.value = Math.max(0, Math.min(page, totalPages.value - 1))
  }

  watch(content, () => { currentPage.value = 0 })

  return {
    pages,
    currentPage,
    isPaginated,
    totalPages,
    currentContent,
    next,
    prev,
    goTo,
  }
}
