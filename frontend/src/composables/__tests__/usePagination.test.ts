import { describe, it, expect } from 'vitest'
import { ref, nextTick } from 'vue'
import { usePagination } from '../usePagination'

describe('usePagination', () => {
  it('桌面模式（containerWidth=1024）不分页', () => {
    const contentRef = ref('a'.repeat(2000))
    const { isPaginated, totalPages, currentContent } = usePagination(
      () => contentRef.value,
      () => 1024,
    )
    expect(isPaginated.value).toBe(false)
    expect(totalPages.value).toBe(1)
    expect(currentContent.value).toBe(contentRef.value)
  })

  it('移动模式（containerWidth=375）按 800 字符分页', () => {
    const contentRef = ref('a'.repeat(2000))
    const { isPaginated, totalPages } = usePagination(
      () => contentRef.value,
      () => 375,
    )
    expect(isPaginated.value).toBe(true)
    // 2000 / 800 = 2.5 → ceil 3
    expect(totalPages.value).toBe(3)
  })

  it('currentPage 0-indexed，初始为 0', () => {
    const contentRef = ref('a'.repeat(2000))
    const { currentPage } = usePagination(() => contentRef.value, () => 375)
    expect(currentPage.value).toBe(0)
  })

  it('next/prev 切换页面并返回 boolean', () => {
    const contentRef = ref('a'.repeat(2000))
    const { currentPage, totalPages, next, prev } = usePagination(
      () => contentRef.value,
      () => 375,
    )
    expect(totalPages.value).toBe(3)
    expect(currentPage.value).toBe(0)

    expect(next()).toBe(true)
    expect(currentPage.value).toBe(1)

    expect(next()).toBe(true)
    expect(currentPage.value).toBe(2)

    // 已经在最后一页
    expect(next()).toBe(false)
    expect(currentPage.value).toBe(2)

    expect(prev()).toBe(true)
    expect(currentPage.value).toBe(1)

    expect(prev()).toBe(true)
    expect(currentPage.value).toBe(0)

    // 已经在第一页
    expect(prev()).toBe(false)
    expect(currentPage.value).toBe(0)
  })

  it('goTo 越界会 clamp', () => {
    const contentRef = ref('a'.repeat(2000))
    const { currentPage, totalPages, goTo } = usePagination(
      () => contentRef.value,
      () => 375,
    )
    expect(totalPages.value).toBe(3)

    goTo(99)
    expect(currentPage.value).toBe(totalPages.value - 1)

    goTo(-1)
    expect(currentPage.value).toBe(0)
  })

  it('currentContent 返回当前页切片', () => {
    const contentRef = ref('a'.repeat(2000))
    const { currentContent, goTo } = usePagination(() => contentRef.value, () => 375)
    // 0-indexed，page 1 对应 [800, 1600)
    goTo(1)
    expect(currentContent.value).toBe('a'.repeat(800))
    expect(currentContent.value.length).toBe(800)

    goTo(2)
    expect(currentContent.value).toBe('a'.repeat(400))
  })

  it('content 变化时 currentPage 重置为 0', async () => {
    const contentRef = ref('a'.repeat(2000))
    const { currentPage, next } = usePagination(() => contentRef.value, () => 375)
    next()
    expect(currentPage.value).toBe(1)

    contentRef.value = 'b'.repeat(2000)
    // usePagination 内部用 watch（默认 pre 模式，异步），需要 nextTick
    await nextTick()
    expect(currentPage.value).toBe(0)
  })
})
