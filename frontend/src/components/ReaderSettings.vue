<script setup lang="ts">
import { useReaderStore } from '@/stores/reader'
import type { ReaderTheme, PageMode } from '@/stores/reader'

const props = defineProps<{ visible: boolean }>()
const emit = defineEmits<{ close: [] }>()

const reader = useReaderStore()

const themes: { value: ReaderTheme; label: string; color: string }[] = [
  { value: 'paper', label: '纸张', color: 'bg-stone-50' },
  { value: 'sepia', label: '护眼', color: 'bg-amber-50' },
  { value: 'white', label: '白色', color: 'bg-white' },
  { value: 'dark', label: '夜间', color: 'bg-gray-900' },
]

const modes: { value: PageMode; label: string }[] = [
  { value: 'scroll', label: '滚动' },
  { value: 'paginate', label: '翻页' },
]

function setTheme(t: ReaderTheme) {
  reader.update({ theme: t })
}
function setMode(m: PageMode) {
  reader.update({ mode: m })
}
function setFontSize(v: number) {
  reader.update({ fontSize: v })
}
function setLineHeight(v: number) {
  reader.update({ lineHeight: v })
}

// 显式引用 props 以避免 unused 警告（模板内通过 visible 访问）
void props
</script>

<template>
  <Teleport to="body">
    <Transition name="fade">
      <div v-if="visible" class="fixed inset-0 z-30">
        <div
          class="absolute inset-0 bg-black/50"
          @click="emit('close')"
        />
        <aside
          class="panel absolute bottom-0 left-0 right-0 max-h-[80vh] overflow-y-auto rounded-t-2xl bg-white p-4 text-gray-900 shadow-xl dark:bg-gray-800 dark:text-gray-100"
        >
          <div class="mb-4 flex items-center justify-between">
            <h2 class="text-base font-medium">阅读设置</h2>
            <button
              type="button"
              aria-label="关闭"
              class="flex h-8 w-8 items-center justify-center rounded text-2xl leading-none hover:bg-black/5 dark:hover:bg-white/10"
              @click="emit('close')"
            >
              ×
            </button>
          </div>

          <div class="space-y-5">
            <!-- 字号 -->
            <div>
              <div
                class="mb-2 flex items-center justify-between text-sm text-gray-600 dark:text-gray-400"
              >
                <span>字号</span>
                <span class="tabular-nums">{{ reader.settings.fontSize }}px</span>
              </div>
              <input
                type="range"
                min="14"
                max="24"
                step="1"
                :value="reader.settings.fontSize"
                class="w-full"
                @input="setFontSize(Number(($event.target as HTMLInputElement).value))"
              >
            </div>

            <!-- 行距 -->
            <div>
              <div
                class="mb-2 flex items-center justify-between text-sm text-gray-600 dark:text-gray-400"
              >
                <span>行距</span>
                <span class="tabular-nums">{{ reader.settings.lineHeight.toFixed(1) }}</span>
              </div>
              <input
                type="range"
                min="1.5"
                max="2.5"
                step="0.1"
                :value="reader.settings.lineHeight"
                class="w-full"
                @input="setLineHeight(Number(($event.target as HTMLInputElement).value))"
              >
            </div>

            <!-- 主题 -->
            <div>
              <div class="mb-2 text-sm text-gray-600 dark:text-gray-400">主题</div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="t in themes"
                  :key="t.value"
                  type="button"
                  class="flex items-center gap-2 rounded px-3 py-1 text-sm transition"
                  :class="
                    reader.settings.theme === t.value
                      ? 'bg-blue-600 text-white'
                      : 'border border-gray-300 hover:bg-gray-50 dark:border-gray-600 dark:hover:bg-gray-700'
                  "
                  @click="setTheme(t.value)"
                >
                  <span
                    class="h-4 w-4 rounded border border-black/10"
                    :class="t.color"
                  />
                  {{ t.label }}
                </button>
              </div>
            </div>

            <!-- 翻页方式 -->
            <div>
              <div class="mb-2 text-sm text-gray-600 dark:text-gray-400">翻页方式</div>
              <div class="flex flex-wrap gap-2">
                <button
                  v-for="m in modes"
                  :key="m.value"
                  type="button"
                  class="rounded px-3 py-1 text-sm transition"
                  :class="
                    reader.settings.mode === m.value
                      ? 'bg-blue-600 text-white'
                      : 'border border-gray-300 hover:bg-gray-50 dark:border-gray-600 dark:hover:bg-gray-700'
                  "
                  @click="setMode(m.value)"
                >
                  {{ m.label }}
                </button>
              </div>
            </div>

            <button
              type="button"
              class="w-full rounded bg-blue-600 px-4 py-2 text-white transition hover:bg-blue-700"
              @click="emit('close')"
            >
              完成
            </button>
          </div>
        </aside>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
.fade-enter-active .panel,
.fade-leave-active .panel {
  transition: transform 0.25s ease;
}
.fade-enter-from .panel,
.fade-leave-to .panel {
  transform: translateY(100%);
}
</style>
