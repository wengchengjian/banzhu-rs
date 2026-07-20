import type { ApiResponse } from '@/types/api'

export class ApiError extends Error {
  constructor(public msg: string, public code: number) {
    super(msg)
    this.name = 'ApiError'
  }
}
export class NetworkError extends Error {
  constructor(message = '网络错误') { super(message); this.name = 'NetworkError' }
}
export class ServerError extends Error {
  constructor(public status: number, message: string) {
    super(message); this.name = 'ServerError'
  }
}

const DEFAULT_TIMEOUT = 30_000

async function request<T>(
  url: string,
  options: RequestInit = {},
  timeout = DEFAULT_TIMEOUT,
): Promise<T> {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), timeout)
  try {
    const res = await fetch(url, { ...options, signal: controller.signal })
    if (!res.ok) {
      const text = await res.text().catch(() => '')
      throw new ServerError(res.status, text || `HTTP ${res.status}`)
    }
    const json: ApiResponse<T> = await res.json()
    if (json.code !== 0) {
      throw new ApiError(json.msg ?? '未知错误', json.code)
    }
    return json.data as T
  } catch (err) {
    if (err instanceof ApiError || err instanceof ServerError) throw err
    if (err instanceof DOMException && err.name === 'AbortError') {
      throw new NetworkError('请求超时')
    }
    throw new NetworkError((err as Error).message)
  } finally {
    clearTimeout(timer)
  }
}

export const client = {
  get: <T>(url: string) => request<T>(url),
  post: <T>(url: string, body?: unknown) =>
    request<T>(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: body !== undefined ? JSON.stringify(body) : undefined,
    }),
  put: <T>(url: string, body?: unknown) =>
    request<T>(url, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: body !== undefined ? JSON.stringify(body) : undefined,
    }),
  delete: <T>(url: string) => request<T>(url, { method: 'DELETE' }),
}
