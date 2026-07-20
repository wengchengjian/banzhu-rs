<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { booksApi } from '@/api/books'
import { progressApi } from '@/api/progress'
import { useReaderStore, type ReaderTheme, type PageMode } from '@/stores/reader'
import { usePagination } from '@/composables/usePagination'
import LoadingSpinner from '@/components/LoadingSpinner.vue'
import EmptyState from '@/components/EmptyState.vue'
import ChapterList from '@/components/ChapterList.vue'
import type { ChapterContent, ChapterListItem } from '@/types/books'

const route = useRoute()
const router = useRouter()
const reader = useReaderStore()

const bookId = computed(() => Number(route.params.bookId))
const chapterOrder = computed(() => Number(route.params.chapterOrder))

const chapter = ref<ChapterContent | null>(null)
const chapters = ref<ChapterListItem[]>([])
const initialLoading = ref(true)
const errorMsg = ref('')

const sideDrawer = ref(false)
const settingsPanel = ref(false)

// 容器宽度（用于 usePagination 判断移动端分页）
const contentRef = ref<HTMLElement | null>(null)
const containerWidth = ref(800)

function updateContainerWidth() {
  if (contentRef.value) {
    containerWidth.value = contentRef.value.clientWidth
  }
}

const {
  currentContent,
  currentPage,
  totalPages,
  isPaginated,
  next: nextPage,
  prev: prevPage,
} = usePagination(
  () => chapter.value?.content ?? '',
  () => containerWidth.value,
)

// 滚动模式直接展示整章；分页模式展示当前页内容
const displayContent = computed(() => {
  if (reader.settings.mode === 'scroll') return chapter.value?.content ?? ''
  return currentContent.value
})

const showFooter = computed(
  () =>
    reader.settings.mode === 'paginate' &&
    isPaginated.value &&
    !initialLoading.value &&
    !errorMsg.value &&
    chapter.value !== null,
)

const themeStyles = computed(() => {
  const t = reader.settings.theme
  const styles: Record<ReaderTheme, string> = {
    paper: 'bg-stone-50 text-stone-900',
    sepia: 'bg-amber-50 text-amber-950',
    white: 'bg-white text-gray-900',
    dark: 'bg-gray-900 text-gray-100',
  }
  return styles[t]
})

const borderClass = computed(() =>
  reader.settings.theme === 'dark' ? 'border-white/10' : 'border-black/10',
)

const contentStyles = computed(() => ({
  fontSize: `${reader.settings.fontSize}px`,
  lineHeight: reader.settings.lineHeight,
}))

const themeLabels: Record<ReaderTheme, string> = {
  paper: '纸张',
  sepia: '护眼',
  white: '白色',
  dark: '夜间',
}

const modeLabels: Record<PageMode, string> = {
  scroll: '滚动',
  paginate: '翻页',
}

const fontSizes = [14, 16, 18, 20, 22, 24]
const lineHeights = [1.5, 1.8, 2.0, 2.5]
const themes: ReaderTheme[] = ['paper', 'sepia', 'white', 'dark']
const modes: PageMode[] = ['scroll', 'paginate']

async function updateProgress(chapterOrderValue: number) {
  try {
    await progressApi.update(bookId.value, {
      chapter_order: chapterOrderValue,
      page_index: 0,
    })
  } catch (e) {
    console.error('Failed to update progress:', e)
  }
}

async function loadChapter(order: number) {
  try {
    const [content, chapterList] = await Promise.all([
      booksApi.chapterContent(bookId.value, order),
      chapters.value.length === 0
        ? booksApi.chapters(bookId.value)
        : Promise.resolve(null),
    ])
    chapter.value = content
    if (chapterList) chapters.value = chapterList.items
    // 切换章节时清空旧错误，避免误显示上一章的错误
    errorMsg.value = ''
    // 重置滚动位置到顶部，避免新章节停留在旧章节的滚动位置
    await nextTick()
    if (contentRef.value) contentRef.value.scrollTo(0, 0)
    await updateProgress(order)
  } catch (e) {
    errorMsg.value = (e as Error).message
  }
}

function goToChapter(order: number) {
  router.push(`/read/${bookId.value}/${order}`)
  sideDrawer.value = false
}

function prevChapter() {
  if (chapter.value?.prev_order) {
    goToChapter(chapter.value.prev_order)
  }
}

function nextChapter() {
  if (chapter.value?.next_order) {
    goToChapter(chapter.value.next_order)
  }
}

// 触摸滑动翻章节（仅 paginate 模式；分页内翻页用底部按钮）
let touchStartX = 0
function onTouchStart(e: TouchEvent) {
  touchStartX = e.touches[0].clientX
}
function onTouchEnd(e: TouchEvent) {
  if (reader.settings.mode !== 'paginate') return
  const deltaX = e.changedTouches[0].clientX - touchStartX
  if (deltaX > 50) prevChapter()
  else if (deltaX < -50) nextChapter()
}

function onKeydown(e: KeyboardEvent) {
  if (e.key === 'ArrowLeft') prevChapter()
  else if (e.key === 'ArrowRight') nextChapter()
}

onMounted(async () => {
  await loadChapter(chapterOrder.value)
  initialLoading.value = false
  updateContainerWidth()
  window.addEventListener('resize', updateContainerWidth)
  window.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  window.removeEventListener('resize', updateContainerWidth)
  window.removeEventListener('keydown', onKeydown)
})

// 监听 route.params.chapterOrder 变化重新加载
watch(chapterOrder, (newOrder) => {
  if (newOrder) loadChapter(newOrder)
})
</script>

<template>
  <div class="flex h-screen flex-col" :class="themeStyles">
    <!-- 顶部栏 -->
    <header
      class="sticky top-0 z-10 flex items-center justify-between border-b px-4 py-3"
      :class="borderClass"
    >
      <div class="flex min-w-0 items-center gap-3">
        <button
          type="button"
          aria-label="章节列表"
          class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded hover:bg-black/5"
          @click="sideDrawer = true"
        >
          ☰
        </button>
        <h1 class="truncate text-base font-medium">
          {{ chapter?.title ?? '加载中...' }}
        </h1>
      </div>
      <button
        type="button"
        aria-label="阅读设置"
        class="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded hover:bg-black/5"
        @click="settingsPanel = true"
      >
        ⚙
      </button>
    </header>

    <!-- 内容区 -->
    <main
      ref="contentRef"
      class="flex-1 overflow-y-auto px-4 py-6"
      @touchstart="onTouchStart"
      @touchend="onTouchEnd"
    >
      <LoadingSpinner v-if="initialLoading" />
      <EmptyState
        v-else-if="errorMsg"
        icon="⚠️"
        :message="`加载失败：${errorMsg}`"
      />
      <div
        v-else-if="chapter"
        class="whitespace-pre-wrap"
        :style="contentStyles"
      >
        {{ displayContent }}
      </div>
    </main>

    <!-- 底部栏（paginate 模式下显示） -->
    <footer
      v-if="showFooter"
      class="flex items-center justify-between border-t px-4 py-2"
      :class="borderClass"
    >
      <button
        type="button"
        class="rounded border px-3 py-1 text-sm transition disabled:cursor-not-allowed disabled:opacity-40"
        :class="borderClass"
        :disabled="currentPage <= 0"
        @click="prevPage"
      >
        上一页
      </button>
      <span class="text-sm tabular-nums">
        {{ currentPage + 1 }} / {{ totalPages }}
      </span>
      <button
        type="button"
        class="rounded border px-3 py-1 text-sm transition disabled:cursor-not-allowed disabled:opacity-40"
        :class="borderClass"
        :disabled="currentPage >= totalPages - 1"
        @click="nextPage"
      >
        下一页
      </button>
    </footer>

    <!-- 侧边抽屉：章节列表 -->
    <Teleport to="body">
      <div v-if="sideDrawer" class="fixed inset-0 z-30">
        <div class="absolute inset-0 bg-black/40" @click="sideDrawer = false" />
        <aside
          class="absolute left-0 top-0 h-full w-80 max-w-[80%] overflow-y-auto bg-white text-gray-900 shadow-xl"
        >
          <div
            class="flex items-center justify-between border-b border-gray-200 px-4 py-3"
          >
            <h2 class="text-base font-medium">章节列表</h2>
            <button
              type="button"
              aria-label="关闭"
              class="flex h-8 w-8 items-center justify-center rounded text-2xl leading-none hover:bg-gray-100"
              @click="sideDrawer = false"
            >
              ×
            </button>
          </div>
          <div class="p-2">
            <ChapterList
              :chapters="chapters"
              :current-order="chapterOrder"
              @select="goToChapter"
            />
          </div>
        </aside>
      </div>
    </Teleport>

    <!-- 设置面板（内联简化版，Task 14 将抽取为 ReaderSettings 组件） -->
    <Teleport to="body">
      <div v-if="settingsPanel" class="fixed inset-0 z-30">
        <div
          class="absolute inset-0 bg-black/40"
          @click="settingsPanel = false"
        />
        <aside
          class="absolute right-0 top-0 h-full w-80 max-w-[80%] overflow-y-auto bg-white text-gray-900 shadow-xl"
        >
          <div
            class="flex items-center justify-between border-b border-gray-200 px-4 py-3"
          >
            <h2 class="text-base font-medium">阅读设置</h2>
            <button
              type="button"
              aria-label="关闭"
              class="flex h-8 w-8 items-center justify-center rounded text-2xl leading-none hover:bg-gray-100"
              @click="settingsPanel = false"
            >
              ×
            </button>
          </div>
          <div class="space-y-5 p-4">
            <!-- 字号 -->
            <div>
              <div class="mb-2 text-sm text-gray-600">字号</div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="size in fontSizes"
                  :key="size"
                  type="button"
                  class="rounded px-3 py-1 text-sm transition"
                  :class="
                    reader.settings.fontSize === size
                      ? 'bg-blue-600 text-white'
                      : 'border border-gray-300 hover:bg-gray-50'
                  "
                  @click="reader.update({ fontSize: size })"
                >
                  {{ size }}
                </button>
              </div>
            </div>
            <!-- 行距 -->
            <div>
              <div class="mb-2 text-sm text-gray-600">行距</div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="lh in lineHeights"
                  :key="lh"
                  type="button"
                  class="rounded px-3 py-1 text-sm transition"
                  :class="
                    reader.settings.lineHeight === lh
                      ? 'bg-blue-600 text-white'
                      : 'border border-gray-300 hover:bg-gray-50'
                  "
                  @click="reader.update({ lineHeight: lh })"
                >
                  {{ lh }}
                </button>
              </div>
            </div>
            <!-- 主题 -->
            <div>
              <div class="mb-2 text-sm text-gray-600">主题</div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="t in themes"
                  :key="t"
                  type="button"
                  class="rounded px-3 py-1 text-sm transition"
                  :class="
                    reader.settings.theme === t
                      ? 'bg-blue-600 text-white'
                      : 'border border-gray-300 hover:bg-gray-50'
                  "
                  @click="reader.update({ theme: t })"
                >
                  {{ themeLabels[t] }}
                </button>
              </div>
            </div>
            <!-- 翻页方式 -->
            <div>
              <div class="mb-2 text-sm text-gray-600">翻页方式</div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="m in modes"
                  :key="m"
                  type="button"
                  class="rounded px-3 py-1 text-sm transition"
                  :class="
                    reader.settings.mode === m
                      ? 'bg-blue-600 text-white'
                      : 'border border-gray-300 hover:bg-gray-50'
                  "
                  @click="reader.update({ mode: m })"
                >
                  {{ modeLabels[m] }}
                </button>
              </div>
            </div>
            <button
              type="button"
              class="w-full rounded bg-blue-600 px-4 py-2 text-white transition hover:bg-blue-700"
              @click="settingsPanel = false"
            >
              完成
            </button>
          </div>
        </aside>
      </div>
    </Teleport>
  </div>
</template>
