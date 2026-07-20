<script setup lang="ts">
import { computed } from 'vue'
import { useThemeStore } from '@/stores/theme'
import type { HeatmapPoint } from '@/api/stats'

const props = defineProps<{ data: HeatmapPoint[]; year: number }>()

const theme = useThemeStore()

const LIGHT_COLORS = ['#ebedf0', '#9be9a8', '#40c463', '#30a14e', '#216e39']
const DARK_COLORS = ['#161b22', '#0e4429', '#006d32', '#26a641', '#39d353']

// 周日开始：行 0=周日, 1=周一, 2=周二, 3=周三, 4=周四, 5=周五, 6=周六
// 只显示 一/三/五
const WEEKDAY_LABELS = ['', '一', '', '三', '', '五', '']
const MONTH_NAMES = [
  '1月', '2月', '3月', '4月', '5月', '6月',
  '7月', '8月', '9月', '10月', '11月', '12月',
]

interface Cell {
  date: string
  level: number
  durationSec: number
  chapters: number
}

function formatISO(d: Date): string {
  const y = d.getFullYear()
  const m = String(d.getMonth() + 1).padStart(2, '0')
  const day = String(d.getDate()).padStart(2, '0')
  return `${y}-${m}-${day}`
}

function getLevel(durationSec: number): number {
  const m = durationSec / 60
  if (m <= 0) return 0
  if (m <= 15) return 1
  if (m <= 30) return 2
  if (m <= 60) return 3
  return 4
}

// 生成 year 年的所有日期 + level
const cells = computed<Cell[]>(() => {
  const dataMap = new Map(props.data.map((p) => [p.date, p]))
  const result: Cell[] = []
  const start = new Date(props.year, 0, 1)
  const end = new Date(props.year, 11, 31)
  for (let d = new Date(start); d <= end; d.setDate(d.getDate() + 1)) {
    const iso = formatISO(d)
    const point = dataMap.get(iso)
    result.push({
      date: iso,
      level: getLevel(point?.duration_sec ?? 0),
      durationSec: point?.duration_sec ?? 0,
      chapters: point?.chapters_read ?? 0,
    })
  }
  return result
})

// 动态列数 × 7 行（周日起始，GitHub 风格）
// 前置 null 让 1/1 落在 firstWeekday 位置；补齐到 columnCount * 7 格
// 通常 53 列；闰年且 1/1 为周六时为 54 列（366 + 6 = 372 > 371）
const grid = computed<(Cell | null)[][]>(() => {
  const firstWeekday = new Date(props.year, 0, 1).getDay() // 0=周日
  const padded: (Cell | null)[] = []
  for (let i = 0; i < firstWeekday; i++) padded.push(null)
  padded.push(...cells.value)
  const columnCount = Math.ceil(padded.length / 7)
  while (padded.length < columnCount * 7) padded.push(null)

  const columns: (Cell | null)[][] = []
  for (let i = 0; i < columnCount; i++) {
    columns.push(padded.slice(i * 7, (i + 1) * 7))
  }
  return columns
})

// 月份标签：对每个月，计算其 1 号落在哪一列
const monthLabels = computed<{ col: number; label: string }[]>(() => {
  const labels: { col: number; label: string }[] = []
  const firstDay = new Date(props.year, 0, 1)
  const firstWeekday = firstDay.getDay()
  for (let m = 0; m < 12; m++) {
    const firstOfMonth = new Date(props.year, m, 1)
    const dayOfYear = Math.floor(
      (firstOfMonth.getTime() - firstDay.getTime()) / 86_400_000,
    )
    const col = Math.floor((dayOfYear + firstWeekday) / 7)
    labels.push({ col, label: MONTH_NAMES[m] })
  }
  return labels
})

function monthLabelAt(col: number): string {
  return monthLabels.value.find((l) => l.col === col)?.label ?? ''
}

function weekdayLabel(row: number): string {
  return WEEKDAY_LABELS[row] ?? ''
}

const colors = computed(() => (theme.current === 'dark' ? DARK_COLORS : LIGHT_COLORS))

function colorForCell(cell: Cell | null | undefined): string {
  if (!cell) return 'transparent'
  return colors.value[cell.level] ?? colors.value[0]
}

function tooltipForCell(cell: Cell | null | undefined): string {
  if (!cell) return ''
  const minutes = Math.round(cell.durationSec / 60)
  return `${cell.date} - ${minutes}分钟 - ${cell.chapters}章`
}
</script>

<template>
  <div class="heatmap-calendar">
    <div class="heatmap-scroll overflow-x-auto">
      <div
        class="heatmap-grid"
        :style="{ gridTemplateColumns: `20px repeat(${grid.length}, 10px)` }"
      >
        <!-- 第 0 行：左上角占位 + 月份标签（动态列数） -->
        <div class="corner" />
        <div
          v-for="(column, colIdx) in grid"
          :key="`m-${colIdx}`"
          class="month-label"
        >
          {{ monthLabelAt(colIdx) }}
        </div>

        <!-- 第 1-7 行：星期标签 + 动态列数 cell -->
        <template v-for="row in 7" :key="`r-${row}`">
          <div class="weekday-label">
            {{ weekdayLabel(row - 1) }}
          </div>
          <div
            v-for="(column, colIdx) in grid"
            :key="`c-${colIdx}-${row}`"
            class="cell"
            :style="{ backgroundColor: colorForCell(grid[colIdx]?.[row - 1]) }"
            :title="tooltipForCell(grid[colIdx]?.[row - 1])"
          />
        </template>
      </div>
    </div>

    <!-- 图例 -->
    <div class="legend">
      <span class="legend-text">少</span>
      <div
        v-for="i in 5"
        :key="i"
        class="legend-cell"
        :style="{ backgroundColor: colors[i - 1] }"
      />
      <span class="legend-text">多</span>
    </div>
  </div>
</template>

<style scoped>
.heatmap-calendar {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.heatmap-grid {
  display: grid;
  grid-template-rows: 16px repeat(7, 10px);
  gap: 2px;
  min-width: min-content;
}

.corner {
  width: 20px;
  height: 16px;
}

.month-label {
  font-size: 10px;
  line-height: 16px;
  height: 16px;
  color: #6b7280;
  white-space: nowrap;
}

.weekday-label {
  font-size: 10px;
  line-height: 10px;
  width: 20px;
  padding-right: 4px;
  text-align: right;
  color: #6b7280;
}

.cell {
  width: 10px;
  height: 10px;
  border-radius: 2px;
}

.legend {
  display: flex;
  align-items: center;
  gap: 4px;
}

.legend-text {
  font-size: 10px;
  color: #6b7280;
}

.legend-cell {
  width: 10px;
  height: 10px;
  border-radius: 2px;
}
</style>
