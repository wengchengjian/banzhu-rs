/// 格式化字数：12345 -> "1.2万字"
export function formatWordCount(n: number): string {
  if (n >= 10000) return `${(n / 10000).toFixed(1)}万字`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}千字`
  return `${n}字`
}

/// 格式化日期：1700000000 -> "2023-11-14"
export function formatDate(ts: number): string {
  return new Date(ts * 1000).toLocaleDateString('zh-CN')
}

/// 格式化相对时间：1700000000 -> "2小时前"
export function formatRelativeTime(ts: number): string {
  const now = Date.now() / 1000
  const diff = now - ts
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)}分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)}小时前`
  if (diff < 2592000) return `${Math.floor(diff / 86400)}天前`
  return formatDate(ts)
}

/// 格式化数字：12345 -> "1.2万"
export function formatNumber(n: number): string {
  if (n >= 10000) return `${(n / 10000).toFixed(1)}万`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}千`
  return String(n)
}
