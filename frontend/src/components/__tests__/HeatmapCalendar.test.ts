import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia } from 'pinia'
import HeatmapCalendar from '../HeatmapCalendar.vue'
import type { HeatmapPoint } from '@/api/stats'

const DATA: HeatmapPoint[] = [
  { date: '2026-01-01', duration_sec: 600, chapters_read: 1 }, // 10 min → level 1
  { date: '2026-01-02', duration_sec: 3600, chapters_read: 5 }, // 60 min → level 3 (<=60)
  { date: '2026-06-15', duration_sec: 120, chapters_read: 0 }, // 2 min → level 1
]

// jsdom 会把 hex 颜色规范化为 rgb，所以同时接受 hex 和 rgb 两种格式
function buildColorRegex(hex: string): RegExp {
  const r = parseInt(hex.slice(1, 3), 16)
  const g = parseInt(hex.slice(3, 5), 16)
  const b = parseInt(hex.slice(5, 7), 16)
  return new RegExp(`${hex}|rgb\\(${r},\\s*${g},\\s*${b}\\)`, 'i')
}

describe('HeatmapCalendar', () => {
  const mountHeatmap = (data: HeatmapPoint[], year = 2026) =>
    mount(HeatmapCalendar, {
      props: { data, year },
      global: { plugins: [createPinia()] },
    })

  it('2026 年渲染 371 个 cell（53 列 × 7 行）', () => {
    const wrapper = mountHeatmap(DATA)
    const cells = wrapper.findAll('.cell')
    expect(cells.length).toBe(53 * 7)
  })

  it('有数据的日子通过 inline style 设置背景色', () => {
    const wrapper = mountHeatmap(DATA)
    const allCells = wrapper.findAll('.cell')
    const jan1Cell = allCells.find((c) => c.attributes('title')?.startsWith('2026-01-01'))
    expect(jan1Cell).toBeDefined()
    // 1/1: 600s = 10min → level 1 → LIGHT_COLORS[1] = '#9be9a8'（默认 light 主题）
    expect(jan1Cell!.attributes('style')).toMatch(buildColorRegex('#9be9a8'))
  })

  it('无数据的日子背景色为 level 0（#ebedf0）', () => {
    const wrapper = mountHeatmap(DATA)
    const allCells = wrapper.findAll('.cell')
    const jan3Cell = allCells.find((c) => c.attributes('title')?.startsWith('2026-01-03'))
    expect(jan3Cell).toBeDefined()
    // 1/3 无数据 → level 0 → LIGHT_COLORS[0] = '#ebedf0'
    expect(jan3Cell!.attributes('style')).toMatch(buildColorRegex('#ebedf0'))
  })

  it('tooltip 包含日期、分钟、章节数', () => {
    const wrapper = mountHeatmap(DATA)
    const allCells = wrapper.findAll('.cell')
    const jan2Cell = allCells.find((c) => c.attributes('title')?.startsWith('2026-01-02'))
    expect(jan2Cell).toBeDefined()
    // 1/2: 3600s = 60min → "2026-01-02 - 60分钟 - 5章"
    expect(jan2Cell!.attributes('title')).toBe('2026-01-02 - 60分钟 - 5章')
  })

  it('1/1 tooltip 包含 10 分钟 1 章', () => {
    const wrapper = mountHeatmap(DATA)
    const allCells = wrapper.findAll('.cell')
    const jan1Cell = allCells.find((c) => c.attributes('title')?.startsWith('2026-01-01'))
    expect(jan1Cell).toBeDefined()
    expect(jan1Cell!.attributes('title')).toBe('2026-01-01 - 10分钟 - 1章')
  })

  it('60 分钟档位为 level 3（#30a14e）', () => {
    const wrapper = mountHeatmap(DATA)
    const allCells = wrapper.findAll('.cell')
    const jan2Cell = allCells.find((c) => c.attributes('title')?.startsWith('2026-01-02'))
    expect(jan2Cell).toBeDefined()
    expect(jan2Cell!.attributes('style')).toMatch(buildColorRegex('#30a14e'))
  })
})
