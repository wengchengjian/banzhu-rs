# 前端技术栈迁移实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 banzhu-spider 前端从 vanilla JS 迁移到 Vue 3 + TS + Vite，引入 SSE、PWA、阅读统计，保留 Rust+Axum+SQLite 后端，单 binary 部署。

**Architecture:** Vue 3 工程位于 `crates/banzhu-spider/frontend/`，构建产物通过 rust-embed 嵌入 binary；后端新增 SSE 流式推送爬虫事件；前端用 Pinia 管理状态，IndexedDB 永久缓存章节支持离线阅读；新增阅读会话上报和 GitHub 风格热力图统计。

**Tech Stack:** Vue 3 + `<script setup>` + TypeScript + Vite + Pinia + Vue Router + Tailwind CSS 4 + @tanstack/vue-virtual + vite-plugin-pwa + ts-rs + rust-embed + Axum SSE + tokio::sync::broadcast

**关联 spec:** `docs/superpowers/specs/2026-07-20-frontend-migration-design.md`

---

## 与 spec 的偏差（已确认）

- **不创建独立的 `stats` Pinia store**：StatsView 直接调用 `statsApi`，数据本地化在组件 ref 中。理由：统计页数据每次访问都重新拉取，无跨组件共享需求，单独 store 增加复杂度但无收益。
- **不创建 `useReader.ts` composable**：其职责（阅读进度追踪、章节切换、会话时长）已由 `stores/reader.ts`（设置持久化）+ `stores/readingSession.ts`（会话计时上报）+ `composables/usePagination.ts`（分页逻辑）三方分担。
- **不创建 `useTheme.ts` composable**：主题逻辑直接由 `stores/theme.ts` 处理（apply + toggle），无需额外 composable 包装。
- **采用 IndexedDB + Workbox SW Cache API 混合方案**：spec 倾向 IndexedDB 单一方案，但 Workbox 的 SW 缓存对 `fetch` 事件拦截更原生（无需手动改写业务代码）。IndexedDB 仍保留用于按书删除（`useChapterCache.ts`），SW Cache 通过 `caches` API 联动删除。

---

## 阶段总览

| 阶段 | Tasks | 产物 |
|------|-------|------|
| P0 脚手架 | Task 1-5 | 可访问空白页 + 基础组件 + 全局错误兜底 |
| P1 后端 SSE | Task 6-9 | 后端 API 调通 |
| P2 核心视图 | Task 10-12 | 可浏览/搜索书籍 |
| P3 阅读体验 | Task 13-15 | 可阅读 |
| P4 书架 + 爬虫 | Task 16-19 | 可管理 |
| P5 统计 + 目标 | Task 20-21 | 可统计 |
| P6 PWA | Task 22-24 | 可离线 |
| P7 测试 + 切换 + 清理 | Task 25-28 | 上线 |

---

## 通用约定

**开发流程**（每个 Task 通用）：
1. 写测试/类型检查
2. 运行验证失败
3. 实现代码
4. 运行验证通过
5. `git add <具体文件>` + `git commit -m "..."`（中文消息，遵循 `feat/fix/docs/refactor` 前缀）

**前端开发命令**：
- 构建：`cd crates/banzhu-spider/frontend && pnpm build`
- 类型检查：`pnpm typecheck`
- 单测：`pnpm test`
- dev server：`pnpm dev`（端口 5173，代理 `/api/*` 到 `http://127.0.0.1:3000`）

**后端开发命令**：
- 构建：`cd crates/banzhu-spider && cargo build`
- 测试：`cargo test`
- 运行：`cargo run`（监听 3000 端口）

**技术约束**（参考 user_profile）：
- Rust 代码遵循 `rust-best-practices` 和 `rust-async-patterns` skill
- Vue 代码遵循 `vue-best-practices` skill（Composition API + `<script setup>` + TypeScript）
- TypeScript 遵循 `typescript-advanced-types` skill
- PowerShell 不支持 heredoc，git commit 用多个 `-m` 参数
- 所有命令应有超时限制

---

## P0：脚手架

### Task 1: 创建 frontend/ Vue + Vite + TS 工程

**Files:**
- Create: `crates/banzhu-spider/frontend/package.json`
- Create: `crates/banzhu-spider/frontend/vite.config.ts`
- Create: `crates/banzhu-spider/frontend/tsconfig.json`
- Create: `crates/banzhu-spider/frontend/tsconfig.node.json`
- Create: `crates/banzhu-spider/frontend/index.html`
- Create: `crates/banzhu-spider/frontend/tailwind.config.ts`
- Create: `crates/banzhu-spider/frontend/postcss.config.js`
- Create: `crates/banzhu-spider/frontend/src/main.ts`
- Create: `crates/banzhu-spider/frontend/src/App.vue`
- Create: `crates/banzhu-spider/frontend/src/assets/styles/main.css`
- Create: `crates/banzhu-spider/frontend/.gitignore`
- Modify: `.gitignore`（添加 `crates/banzhu-spider/frontend/node_modules/` 和 `crates/banzhu-spider/frontend/dist/`）

- [ ] **Step 1: 创建 package.json**

依赖版本：
```json
{
  "name": "banzhu-spider-frontend",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "vue-tsc -b && vite build",
    "preview": "vite preview",
    "test": "vitest run",
    "test:watch": "vitest",
    "typecheck": "vue-tsc --noEmit"
  },
  "dependencies": {
    "vue": "^3.5.13",
    "vue-router": "^4.5.0",
    "pinia": "^2.3.0",
    "@tanstack/vue-virtual": "^3.11.3"
  },
  "devDependencies": {
    "@vitejs/plugin-vue": "^5.2.1",
    "vite": "^6.0.7",
    "vue-tsc": "^2.2.0",
    "typescript": "^5.7.3",
    "tailwindcss": "^4.0.0",
    "@tailwindcss/postcss": "^4.0.0",
    "postcss": "^8.4.49",
    "vitest": "^2.1.8",
    "@vue/test-utils": "^2.4.6",
    "jsdom": "^25.0.1",
    "vite-plugin-pwa": "^0.21.1"
  }
}
```

- [ ] **Step 2: 创建 vite.config.ts**

```typescript
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [vue()],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    port: 5173,
    proxy: { '/api': { target: 'http://127.0.0.1:3000', changeOrigin: true } },
  },
  build: { outDir: 'dist', emptyOutDir: true },
})
```

- [ ] **Step 3: 创建 tsconfig.json + tsconfig.node.json**

`tsconfig.json`：
```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "jsx": "preserve",
    "sourceMap": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "esModuleInterop": true,
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "skipLibCheck": true,
    "noEmit": true,
    "baseUrl": ".",
    "paths": { "@/*": ["./src/*"] }
  },
  "include": ["src/**/*.ts", "src/**/*.tsx", "src/**/*.vue"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

`tsconfig.node.json`：
```json
{
  "compilerOptions": {
    "composite": true,
    "skipLibCheck": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true
  },
  "include": ["vite.config.ts", "tailwind.config.ts"]
}
```

- [ ] **Step 4: 创建 index.html**

```html
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover">
  <title>版主网 - 简洁无广告小说阅读</title>
</head>
<body>
  <div id="app"></div>
  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

- [ ] **Step 5: 创建 tailwind.config.ts + postcss.config.js + main.css**

`tailwind.config.ts`：
```typescript
import type { Config } from 'tailwindcss'
export default {
  content: ['./index.html', './src/**/*.{vue,ts,tsx}'],
  darkMode: 'class',
  theme: { extend: {
    colors: { reader: { paper: '#f8f5e9', sepia: '#f4ecd8', dark: '#1a1a1a' } },
  }},
  plugins: [],
} satisfies Config
```

`postcss.config.js`：
```javascript
export default { plugins: { '@tailwindcss/postcss': {} } }
```

`src/assets/styles/main.css`：
```css
@import 'tailwindcss';
:root { color-scheme: light dark; }
body {
  margin: 0;
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', 'PingFang SC', 'Microsoft YaHei', sans-serif;
  -webkit-font-smoothing: antialiased;
}
html.dark body { background-color: #0d1117; color: #c9d1d9; }
```

- [ ] **Step 6: 创建最小可运行的 main.ts + App.vue**

`src/main.ts`：
```typescript
import { createApp } from 'vue'
import App from './App.vue'
import './assets/styles/main.css'
createApp(App).mount('#app')
```

`src/App.vue`：
```vue
<script setup lang="ts">
const message = 'Hello Vue 3 + Vite'
</script>
<template>
  <div class="container mx-auto p-4">
    <h1 class="text-2xl font-bold">{{ message }}</h1>
  </div>
</template>
```

- [ ] **Step 7: 创建 .gitignore 并更新根 .gitignore**

`crates/banzhu-spider/frontend/.gitignore`：
```
node_modules/
dist/
*.log
.DS_Store
.vite/
```

在项目根 `.gitignore` 追加：
```
crates/banzhu-spider/frontend/node_modules/
crates/banzhu-spider/frontend/dist/
```

- [ ] **Step 8: 安装依赖并启动验证**

Run:
```bash
cd crates/banzhu-spider/frontend
pnpm install
pnpm dev
```
Expected: Vite dev server 启动，访问 http://localhost:5173 显示 "Hello Vue 3 + Vite"

- [ ] **Step 9: Commit**

```bash
git add crates/banzhu-spider/frontend/ .gitignore
git commit -m "feat(frontend): 初始化 Vue 3 + Vite + TS 工程"
```

---

### Task 2: rust-embed 接入 + build.rs

**Files:**
- Modify: `crates/banzhu-spider/Cargo.toml`
- Create: `crates/banzhu-spider/build.rs`
- Modify: `crates/banzhu-spider/src/web/mod.rs`

- [ ] **Step 1: Cargo.toml 添加依赖**

```toml
[dependencies]
rust-embed = { version = "8", features = ["axum"] }
mime_guess = "2"
```

- [ ] **Step 2: 创建 build.rs**

```rust
use std::path::Path;
fn main() {
    let dist = Path::new("frontend/dist/index.html");
    if !dist.exists() {
        println!("cargo:warning=frontend/dist/index.html 不存在。请先运行：cd frontend && pnpm build");
    }
    println!("cargo:rerun-if-changed=frontend/dist/index.html");
}
```

- [ ] **Step 3: 在 src/web/mod.rs 添加 rust-embed 处理器**

```rust
use axum::{
    body::Body,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAsset;

pub async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = FrontendAsset::get(path) {
        return file_response(path, file);
    }
    // SPA fallback
    if let Some(file) = FrontendAsset::get("index.html") {
        return file_response("index.html", file);
    }
    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

fn file_response(path: &str, file: rust_embed::EmbeddedFile) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, mime.as_ref())],
        Body::from(file.data.into_owned()),
    ).into_response()
}
```

- [ ] **Step 4: 路由层替换 ServeDir 为 fallback**

定位 `src/banzhuspider.rs` 或 `src/main.rs` 中的 Router 构造，把 `.nest_service("/", ServeDir::new("static"))` 改为：

```rust
.fallback(crate::web::static_handler)
```

- [ ] **Step 5: 构建前端 + 后端验证**

Run:
```bash
cd crates/banzhu-spider/frontend && pnpm build
cd .. && cargo build && cargo run
```
Expected: 访问 http://127.0.0.1:3000/ 显示 Vue 页面

- [ ] **Step 6: Commit**

```bash
git add crates/banzhu-spider/Cargo.toml crates/banzhu-spider/build.rs crates/banzhu-spider/src/
git commit -m "feat(web): 接入 rust-embed 嵌入前端静态资源"
```

---

### Task 3: 前端基础（路由 + Pinia + AppHeader + theme store + 占位视图）

**Files:**
- Create: `crates/banzhu-spider/frontend/src/router/index.ts`
- Create: `crates/banzhu-spider/frontend/src/stores/theme.ts`
- Create: `crates/banzhu-spider/frontend/src/components/AppHeader.vue`
- Create: `crates/banzhu-spider/frontend/src/views/{Home,BookDetail,Search,Reader,Shelf,Crawler,Stats,Settings,NotFound}View.vue`（9 个占位 SFC）
- Modify: `crates/banzhu-spider/frontend/src/main.ts`
- Modify: `crates/banzhu-spider/frontend/src/App.vue`

- [ ] **Step 1: 创建 src/stores/theme.ts**

```typescript
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
```

- [ ] **Step 2: 创建 src/router/index.ts**

定义 8 个路由（`/`、`/book/:id`、`/search`、`/read/:bookId/:chapterOrder`、`/shelf`、`/crawler`、`/stats`、`/settings`）+ 404 兜底，全部用动态 import。`history: createWebHistory()`，`scrollBehavior`：savedPosition 优先，否则 `{ top: 0 }`。

- [ ] **Step 3: 创建 src/components/AppHeader.vue**

顶栏组件，包含：站名（RouterLink to `/`）、导航（首页/书架/爬虫/统计/设置，hidden md:flex）、搜索框（enter 跳 `/search?q=`）、主题切换按钮。用 Tailwind 类，dark: 变体。

- [ ] **Step 4: 创建 9 个占位视图**

每个 SFC 内容为：
```vue
<script setup lang="ts"></script>
<template>
  <div class="container mx-auto p-4">
    <h1 class="text-2xl">ViewName（待实现）</h1>
  </div>
</template>
```

`NotFoundView.vue`：显示 404 + 返回首页链接。

- [ ] **Step 5: 更新 src/main.ts 挂载 Pinia 和 Router**

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import './assets/styles/main.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)
app.mount('#app')
```

- [ ] **Step 6: 更新 src/App.vue**

```vue
<script setup lang="ts">
import AppHeader from '@/components/AppHeader.vue'
</script>
<template>
  <AppHeader />
  <RouterView />
</template>
```

- [ ] **Step 7: 构建并验证**

Run:
```bash
cd crates/banzhu-spider/frontend && pnpm build
cd .. && cargo run
```
Expected: 8 个路由可切换，主题切换按钮工作，AppHeader 显示

- [ ] **Step 8: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): 接入 Vue Router + Pinia + AppHeader + theme store"
```

---

### Task 4: API 客户端 + ts-rs 类型生成

**Files:**
- Modify: `crates/banzhu-spider/Cargo.toml`
- Modify: `crates/banzhu-spider/src/db/models.rs`
- Create: `crates/banzhu-spider/frontend/src/api/client.ts`
- Create: `crates/banzhu-spider/frontend/src/types/api.ts`（聚合 re-export）
- 自动生成: `crates/banzhu-spider/frontend/src/types/api/*.ts`

- [ ] **Step 1: Cargo.toml 添加 ts-rs**

```toml
[dependencies]
ts-rs = { version = "10", features = ["chrono-impl", "no-serde-warnings"] }
```

- [ ] **Step 2: 在 src/db/models.rs 为关键模型加 TS 导出**

为现有模型 `BookRecord`、`ChapterRecord`、`SectionRecord`、`BookshelfRecord`、`ReadingProgressRecord`、`CrawlLogRecord`、`CrawlTaskRecord` 添加 `TS` derive 和 `#[ts(export, export_to = "../frontend/src/types/api/")]`。示例：

```rust
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
#[serde(rename_all = "snake_case")]
pub struct BookRecord {
    pub id: i64,
    pub website_book_id: Option<i64>,
    pub path_num: i64,
    pub title: String,
    pub filename: String,
    pub author: String,
    pub category: String,
    pub introduce: String,
    pub likes: i64,
    pub word_count: i64,
    pub page_count: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
```

新增两个模型（字段完整给出，避免类型漂移）：

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
#[serde(rename_all = "snake_case")]
pub struct ReadingSessionRecord {
    pub id: i64,
    pub book_id: i64,
    pub chapter_order: i64,
    pub duration_sec: i64,
    pub chapters_read: i64,
    pub started_at: i64,
    pub ended_at: i64,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "../frontend/src/types/api/")]
#[serde(rename_all = "snake_case")]
pub struct ReadingGoalRecord {
    pub id: i64,
    pub daily_minutes: i64,
    pub daily_chapters: i64,
    pub updated_at: i64,
}
```

同时为 `ReadingProgressRecord` 增加 `last_read_at: i64` 字段（对应 Task 6 的 ALTER TABLE）。

- [ ] **Step 3: 创建 frontend/src/types/api.ts（聚合文件）**

```typescript
export type { BookRecord } from './api/BookRecord'
export type { ChapterRecord } from './api/ChapterRecord'
export type { SectionRecord } from './api/SectionRecord'
export type { BookshelfRecord } from './api/BookshelfRecord'
export type { ReadingProgressRecord } from './api/ReadingProgressRecord'
export type { CrawlLogRecord } from './api/CrawlLogRecord'
export type { CrawlTaskRecord } from './api/CrawlTaskRecord'
export type { ReadingSessionRecord } from './api/ReadingSessionRecord'
export type { ReadingGoalRecord } from './api/ReadingGoalRecord'

export interface ApiResponse<T> { code: number; data?: T; msg?: string }
export interface Paginated<T> { items: T[]; total: number; page: number; limit: number }
```

- [ ] **Step 4: 创建 frontend/src/api/client.ts**

```typescript
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
```

- [ ] **Step 5: 运行 cargo test 生成 TS 类型**

Run:
```bash
cd crates/banzhu-spider && cargo test
```
Expected: `frontend/src/types/api/` 目录生成 `BookRecord.ts` 等文件

- [ ] **Step 6: 类型检查**

Run:
```bash
cd crates/banzhu-spider/frontend && pnpm typecheck
```
Expected: 无错误

- [ ] **Step 7: Commit**

```bash
git add crates/banzhu-spider/Cargo.toml crates/banzhu-spider/src/db/models.rs crates/banzhu-spider/frontend/src/api/ crates/banzhu-spider/frontend/src/types/
git commit -m "feat: 接入 ts-rs 自动生成 TS 类型 + API 客户端封装"
```

---

### Task 5: 基础通用组件 + 全局错误兜底

**Files:**
- Create: `crates/banzhu-spider/frontend/src/components/{EmptyState,LoadingSpinner,StatCard,ToastContainer,ConfirmDialog}.vue`
- Create: `crates/banzhu-spider/frontend/src/composables/useToast.ts`
- Create: `crates/banzhu-spider/frontend/src/composables/useConfirm.ts`
- Modify: `crates/banzhu-spider/frontend/src/main.ts`（注册全局错误处理）
- Modify: `crates/banzhu-spider/frontend/src/App.vue`（挂载 ToastContainer + ConfirmDialog）

- [ ] **Step 1: 创建 EmptyState.vue**

```vue
<script setup lang="ts">
defineProps<{ message?: string; icon?: string }>()
defineEmits<{ retry: [] }>()
</script>
<template>
  <div class="flex flex-col items-center justify-center py-12 text-gray-500">
    <div class="text-4xl mb-2">{{ icon ?? '📭' }}</div>
    <p>{{ message ?? '暂无数据' }}</p>
    <button
      v-if="$listeners.retry"
      class="mt-3 px-4 py-1 text-sm text-blue-500 border border-blue-500 rounded hover:bg-blue-50"
      @click="$emit('retry')"
    >
      重试
    </button>
  </div>
</template>
```

- [ ] **Step 2: 创建 LoadingSpinner.vue**

```vue
<script setup lang="ts">
defineProps<{ message?: string }>()
</script>
<template>
  <div class="flex items-center justify-center py-8 text-gray-400">
    <svg class="animate-spin h-5 w-5 mr-2" viewBox="0 0 24 24" fill="none">
      <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" />
      <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.4 0 0 5.4 0 12h4z" />
    </svg>
    <span>{{ message ?? '加载中...' }}</span>
  </div>
</template>
```

- [ ] **Step 3: 创建 StatCard.vue**

```vue
<script setup lang="ts">
import { computed } from 'vue'
const props = defineProps<{
  label: string
  value: number | string
  total?: number  // 可选，用于进度展示
  unit?: string
}>()
const percent = computed(() => {
  if (props.total === undefined || !props.total) return 0
  return Math.min(100, Math.round((Number(props.value) / props.total) * 100))
})
</script>
<template>
  <div class="p-4 rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800">
    <div class="text-sm text-gray-500 dark:text-gray-400">{{ label }}</div>
    <div class="mt-1 flex items-baseline">
      <span class="text-2xl font-bold">{{ value }}</span>
      <span v-if="total" class="text-sm text-gray-400 ml-1">/ {{ total }}</span>
      <span v-if="unit" class="text-sm text-gray-400 ml-1">{{ unit }}</span>
    </div>
    <div v-if="total !== undefined" class="mt-2 h-1.5 bg-gray-100 rounded-full overflow-hidden">
      <div class="h-full bg-blue-500 transition-all" :style="{ width: `${percent}%` }" />
    </div>
  </div>
</template>
```

- [ ] **Step 4: 创建 useToast composable + ToastContainer.vue**

```typescript
// src/composables/useToast.ts
import { ref } from 'vue'

export type ToastType = 'success' | 'error' | 'info' | 'warning'
export interface ToastItem {
  id: number
  type: ToastType
  message: string
}

const toasts = ref<ToastItem[]>([])
let nextId = 1

export function useToast() {
  function show(message: string, type: ToastType = 'info', duration = 3000) {
    const id = nextId++
    toasts.value.push({ id, type, message })
    setTimeout(() => remove(id), duration)
  }
  function remove(id: number) {
    toasts.value = toasts.value.filter(t => t.id !== id)
  }
  return {
    toasts,
    show,
    remove,
    success: (msg: string) => show(msg, 'success'),
    error: (msg: string) => show(msg, 'error', 5000),
    info: (msg: string) => show(msg, 'info'),
    warning: (msg: string) => show(msg, 'warning', 4000),
  }
}
```

```vue
<!-- src/components/ToastContainer.vue -->
<script setup lang="ts">
import { useToast } from '@/composables/useToast'
const { toasts, remove } = useToast()
const colorMap: Record<string, string> = {
  success: 'bg-green-500',
  error: 'bg-red-500',
  info: 'bg-blue-500',
  warning: 'bg-orange-500',
}
</script>
<template>
  <div class="fixed top-4 right-4 z-50 flex flex-col gap-2">
    <TransitionGroup name="toast">
      <div
        v-for="t in toasts"
        :key="t.id"
        :class="['text-white px-4 py-2 rounded shadow-lg cursor-pointer', colorMap[t.type]]"
        @click="remove(t.id)"
      >
        {{ t.message }}
      </div>
    </TransitionGroup>
  </div>
</template>
<style scoped>
.toast-enter-active, .toast-leave-active { transition: all 0.3s; }
.toast-enter-from, .toast-leave-to { opacity: 0; transform: translateX(20px); }
</style>
```

- [ ] **Step 5: 创建 useConfirm composable + ConfirmDialog.vue**

```typescript
// src/composables/useConfirm.ts
import { ref } from 'vue'

interface ConfirmOptions {
  title?: string
  message: string
  confirmText?: string
  cancelText?: string
}

const visible = ref(false)
const options = ref<ConfirmOptions>({ message: '' })
let resolver: ((value: boolean) => void) | null = null

export function useConfirm() {
  function confirm(opts: ConfirmOptions): Promise<boolean> {
    options.value = opts
    visible.value = true
    return new Promise<boolean>(resolve => { resolver = resolve })
  }
  function resolve(value: boolean) {
    visible.value = false
    resolver?.(value)
    resolver = null
  }
  return { visible, options, confirm, resolve }
}
```

```vue
<!-- src/components/ConfirmDialog.vue -->
<script setup lang="ts">
import { useConfirm } from '@/composables/useConfirm'
const { visible, options, resolve } = useConfirm()
</script>
<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="fixed inset-0 z-50 flex items-center justify-center bg-black/40"
      @click.self="resolve(false)"
    >
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-sm w-full mx-4">
        <h3 class="text-lg font-medium mb-2">{{ options.title ?? '确认操作' }}</h3>
        <p class="text-gray-600 dark:text-gray-300 mb-4">{{ options.message }}</p>
        <div class="flex justify-end gap-2">
          <button class="px-4 py-1.5 text-sm border rounded" @click="resolve(false)">
            {{ options.cancelText ?? '取消' }}
          </button>
          <button class="px-4 py-1.5 text-sm bg-red-500 text-white rounded" @click="resolve(true)">
            {{ options.confirmText ?? '确认' }}
          </button>
        </div>
      </div>
    </div>
  </Teleport>
</template>
```

- [ ] **Step 6: main.ts 注册全局错误兜底**

```typescript
import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import { router } from './router'
import { useToast } from '@/composables/useToast'
import './assets/styles/main.css'

const app = createApp(App)
app.use(createPinia())
app.use(router)

// 全局错误兜底
const toast = useToast()
app.config.errorHandler = (err, _instance, info) => {
  console.error('[Vue Error]', err, info)
  toast.error((err as Error)?.message ?? '应用发生未知错误')
}
window.addEventListener('unhandledrejection', (event) => {
  console.error('[Unhandled Promise]', event.reason)
  toast.error(event.reason?.message ?? '异步操作失败')
})

app.mount('#app')
```

- [ ] **Step 7: App.vue 挂载 ToastContainer + ConfirmDialog**

```vue
<script setup lang="ts">
import AppHeader from '@/components/AppHeader.vue'
import ToastContainer from '@/components/ToastContainer.vue'
import ConfirmDialog from '@/components/ConfirmDialog.vue'
</script>
<template>
  <AppHeader />
  <RouterView />
  <ToastContainer />
  <ConfirmDialog />
</template>
```

- [ ] **Step 8: 类型检查**

Run:
```bash
cd crates/banzhu-spider/frontend && pnpm typecheck
```
Expected: 无错误

- [ ] **Step 9: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): 通用组件 (EmptyState/LoadingSpinner/StatCard/Toast/Confirm) + 全局错误兜底"
```

---

## P1：后端 SSE

### Task 6: DB schema 新增表 + Rust 模型

**Files:**
- Modify: `crates/banzhu-spider/src/db/schema.rs`
- Modify: `crates/banzhu-spider/src/db/crud.rs`

- [ ] **Step 1: schema.rs 添加 reading_sessions 和 reading_goals 建表 SQL**

```rust
pub const CREATE_READING_SESSIONS: &str = "
CREATE TABLE IF NOT EXISTS reading_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL,
    chapter_order INTEGER NOT NULL,
    duration_sec INTEGER NOT NULL CHECK(duration_sec > 0),
    chapters_read INTEGER NOT NULL DEFAULT 0,
    started_at INTEGER NOT NULL,
    ended_at INTEGER NOT NULL,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_reading_sessions_book ON reading_sessions(book_id);
CREATE INDEX IF NOT EXISTS idx_reading_sessions_started ON reading_sessions(started_at DESC);
";

pub const CREATE_READING_GOALS: &str = "
CREATE TABLE IF NOT EXISTS reading_goals (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    daily_minutes INTEGER NOT NULL DEFAULT 30 CHECK(daily_minutes >= 0),
    daily_chapters INTEGER NOT NULL DEFAULT 5 CHECK(daily_chapters >= 0),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
INSERT OR IGNORE INTO reading_goals (id) VALUES (1);
";

pub const ALTER_READING_PROGRESS_LAST_READ: &str =
    "ALTER TABLE reading_progress ADD COLUMN last_read_at INTEGER NOT NULL DEFAULT 0;";
```

- [ ] **Step 2: 在 schema 初始化函数中执行新 SQL**

```rust
conn.execute_batch(CREATE_READING_SESSIONS)?;
conn.execute_batch(CREATE_READING_GOALS)?;

// ALTER TABLE 不支持 IF NOT EXISTS，用 PRAGMA 检查
let cols: Vec<String> = conn
    .prepare("PRAGMA table_info(reading_progress)")?
    .query_map([], |r| r.get::<_, String>(1))?
    .filter_map(Result::ok)
    .collect();
if !cols.iter().any(|c| c == "last_read_at") {
    conn.execute(ALTER_READING_PROGRESS_LAST_READ, [])?;
}
```

- [ ] **Step 3: crud.rs 添加 CRUD 函数**

在 `src/db/crud.rs` 末尾追加以下函数：

```rust
use sqlx::Row;

pub async fn insert_reading_session(
    pool: &sqlx::SqlitePool,
    book_id: i64,
    chapter_order: i64,
    duration_sec: i64,
    chapters_read: i64,
    started_at: i64,
    ended_at: i64,
) -> AppResult<i64> {
    let row = sqlx::query(
        "INSERT INTO reading_sessions (book_id, chapter_order, duration_sec, chapters_read, started_at, ended_at) \
         VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(book_id)
    .bind(chapter_order)
    .bind(duration_sec)
    .bind(chapters_read)
    .bind(started_at)
    .bind(ended_at)
    .fetch_one(pool)
    .await?;
    Ok(row.get::<i64, _>("id"))
}

pub async fn sum_today_reading(pool: &sqlx::SqlitePool) -> AppResult<(i64, i64)> {
    let row = sqlx::query(
        "SELECT COALESCE(SUM(duration_sec), 0) AS duration, \
                COALESCE(SUM(chapters_read), 0) AS chapters \
         FROM reading_sessions \
         WHERE started_at >= unixepoch('today', 'localtime')",
    )
    .fetch_one(pool)
    .await?;
    Ok((row.get::<i64, _>("duration"), row.get::<i64, _>("chapters")))
}

pub async fn heatmap_data(pool: &sqlx::SqlitePool, year: i32) -> AppResult<Vec<(String, i64, i64)>> {
    let start = format!("{}-01-01T00:00:00", year);
    let end = format!("{}-01-01T00:00:00", year + 1);
    let rows = sqlx::query(
        "SELECT date(started_at, 'unixepoch', 'localtime') AS date, \
                COALESCE(SUM(duration_sec), 0) AS duration, \
                COALESCE(SUM(chapters_read), 0) AS chapters \
         FROM reading_sessions \
         WHERE started_at >= strftime('%s', ?1) \
           AND started_at <  strftime('%s', ?2) \
         GROUP BY date",
    )
    .bind(start)
    .bind(end)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter()
        .map(|r| (r.get::<String, _>("date"), r.get::<i64, _>("duration"), r.get::<i64, _>("chapters")))
        .collect())
}

pub async fn reading_timeline(pool: &sqlx::SqlitePool, days: i32) -> AppResult<Vec<(String, i64, i64)>> {
    let rows = sqlx::query(
        "SELECT date(started_at, 'unixepoch', 'localtime') AS date, \
                COALESCE(SUM(duration_sec), 0) AS duration, \
                COALESCE(SUM(chapters_read), 0) AS chapters \
         FROM reading_sessions \
         WHERE started_at >= unixepoch('now', ?1) \
         GROUP BY date \
         ORDER BY date ASC",
    )
    .bind(format!("-{} days", days))
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter()
        .map(|r| (r.get::<String, _>("date"), r.get::<i64, _>("duration"), r.get::<i64, _>("chapters")))
        .collect())
}

pub async fn reading_history(pool: &sqlx::SqlitePool, limit: i64) -> AppResult<Vec<ReadingHistoryRow>> {
    let rows = sqlx::query_as::<_, ReadingHistoryRow>(
        "SELECT rs.book_id, b.title AS book_title, \
                MAX(rs.started_at) AS last_read_at, \
                MAX(rs.chapter_order) AS last_chapter_order, \
                COALESCE(SUM(rs.duration_sec), 0) AS total_duration_sec, \
                COALESCE(SUM(rs.chapters_read), 0) AS chapters_read \
         FROM reading_sessions rs \
         LEFT JOIN books b ON b.id = rs.book_id \
         GROUP BY rs.book_id \
         ORDER BY last_read_at DESC \
         LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_reading_goal(pool: &sqlx::SqlitePool) -> AppResult<ReadingGoalRecord> {
    let row = sqlx::query_as::<_, ReadingGoalRecord>(
        "SELECT id, daily_minutes, daily_chapters, updated_at FROM reading_goals WHERE id = 1",
    )
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn update_reading_goal(
    pool: &sqlx::SqlitePool,
    daily_minutes: i64,
    daily_chapters: i64,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE reading_goals SET daily_minutes = ?, daily_chapters = ?, updated_at = unixepoch() WHERE id = 1",
    )
    .bind(daily_minutes)
    .bind(daily_chapters)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_tasks_by_status(
    pool: &sqlx::SqlitePool,
    status: &str,
    limit: i64,
) -> AppResult<Vec<CrawlTaskRecord>> {
    let rows = sqlx::query_as::<_, CrawlTaskRecord>(
        "SELECT id, website_book_id, book_title, status, started_at, finished_at, \
                chapters_done, chapters_total, error_message \
         FROM crawl_tasks WHERE status = ? ORDER BY started_at DESC LIMIT ?",
    )
    .bind(status)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn reset_task_status(pool: &sqlx::SqlitePool, website_book_id: i64) -> AppResult<u64> {
    let res = sqlx::query(
        "UPDATE crawl_tasks SET status = 'pending', error_message = NULL, \
                started_at = NULL, finished_at = NULL \
         WHERE website_book_id = ? AND status IN ('failed', 'success')",
    )
    .bind(website_book_id)
    .execute(pool)
    .await?;
    Ok(res.rows_affected())
}

pub async fn delete_tasks_by_status(pool: &sqlx::SqlitePool, status: &str) -> AppResult<u64> {
    let res = sqlx::query("DELETE FROM crawl_tasks WHERE status = ?")
        .bind(status)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

pub async fn list_logs_after(
    pool: &sqlx::SqlitePool,
    after_id: i64,
    limit: i64,
) -> AppResult<Vec<CrawlLogRecord>> {
    let rows = sqlx::query_as::<_, CrawlLogRecord>(
        "SELECT id, task_id, level, msg, ts FROM crawl_logs WHERE id > ? ORDER BY id ASC LIMIT ?",
    )
    .bind(after_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn count_all_tasks(pool: &sqlx::SqlitePool) -> AppResult<i64> {
    let row = sqlx::query("SELECT COUNT(*) AS cnt FROM crawl_tasks")
        .fetch_one(pool)
        .await?;
    Ok(row.get::<i64, _>("cnt"))
}

pub async fn list_all_tasks(pool: &sqlx::SqlitePool, limit: i64) -> AppResult<Vec<CrawlTaskRecord>> {
    let rows = sqlx::query_as::<_, CrawlTaskRecord>(
        "SELECT id, website_book_id, book_title, status, started_at, finished_at, \
                chapters_done, chapters_total, error_message \
         FROM crawl_tasks ORDER BY started_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
```

新增 `ReadingHistoryRow` struct（用于 reading_history 返回）：

```rust
#[derive(Debug, Clone, sqlx::FromRow, Serialize)]
pub struct ReadingHistoryRow {
    pub book_id: i64,
    pub book_title: Option<String>,
    pub last_read_at: i64,
    pub last_chapter_order: i64,
    pub total_duration_sec: i64,
    pub chapters_read: i64,
}
```

- [ ] **Step 4: 编译验证**

Run: `cargo build`
Expected: 成功

- [ ] **Step 5: Commit**

```bash
git add crates/banzhu-spider/src/db/
git commit -m "feat(db): 新增 reading_sessions/reading_goals 表和 CRUD"
```

---

### Task 7: 统一 API 响应信封 + AppError

**Files:**
- Modify: `crates/banzhu-spider/Cargo.toml`（添加 `thiserror = "2"`）
- Modify: `crates/banzhu-spider/src/error.rs`
- Modify: `crates/banzhu-spider/src/web/{books,search,crawl,shelf,stats,export}.rs`（包装 handler）

- [ ] **Step 1: 在 src/error.rs 实现 AppError**

```rust
use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found")] NotFound,
    #[error("bad request: {0}")] BadRequest(String),
    #[error("database error: {0}")] Database(#[from] sqlx::Error),
    #[error("internal error: {0}")] Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, _code) = match &self {
            AppError::NotFound => (StatusCode::NOT_FOUND, 1001),
            AppError::BadRequest(_) => (StatusCode::BAD_REQUEST, 1002),
            AppError::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, 1003),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, 1004),
        };
        (status, Json(json!({ "code": -1, "msg": self.to_string() }))).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn ok<T: serde::Serialize>(data: T) -> Json<serde_json::Value> {
    Json(json!({ "code": 0, "data": data }))
}
```

- [ ] **Step 2: 改造所有现有 handler**

对每个 handler：
- 返回类型改为 `AppResult<Json<serde_json::Value>>`
- 用 `?` 替代 `unwrap()`
- 成功用 `Ok(ok(data))` 包装

示例（`src/web/books.rs` 的 `list_books`）：
```rust
pub async fn list_books(
    State(state): State<AppState>,
    Query(params): Query<ListBooksParams>,
) -> AppResult<Json<Value>> {
    let books = crud::list_books(&state.pool, &params).await?;
    Ok(ok(books))
}
```

- [ ] **Step 3: 编译并手动测试**

Run:
```bash
cargo build && cargo run
# 另开终端：
curl http://127.0.0.1:3000/api/books?page=1&limit=5
```
Expected: 返回 `{"code":0,"data":{"items":[...],"total":N}}`

测试 404：
```bash
curl http://127.0.0.1:3000/api/books/999999
```
Expected: `{"code":-1,"msg":"not found"}`

- [ ] **Step 4: Commit**

```bash
git add crates/banzhu-spider/src/ crates/banzhu-spider/Cargo.toml
git commit -m "feat(api): 统一响应信封 {code,data} + AppError 错误处理"
```

---

### Task 8: scheduler broadcast + SSE handler + 批量端点

**Files:**
- Modify: `crates/banzhu-spider/src/task/mod.rs`（定义 CrawlEvent + EventBus）
- Modify: `crates/banzhu-spider/src/banzhuspider.rs` 或 `src/main.rs`（AppState 注入 EventBus）
- Modify: `crates/banzhu-spider/src/scheduler.rs`（emit 事件）
- Modify: `crates/banzhu-spider/src/web/crawl.rs`（SSE handler + 批量端点）
- Modify: `crates/banzhu-spider/src/web/mod.rs`（注册路由）

- [ ] **Step 1: src/task/mod.rs 定义 CrawlEvent 和 EventBus**

```rust
use serde::Serialize;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlEvent {
    Status { running: bool, current_page: i64, books_found: i64, books_downloaded: i64, books_failed: i64, message: String },
    TaskFull { tasks: Vec<serde_json::Value> },
    TaskUpdate { task: serde_json::Value },
    Log { id: i64, level: String, message: String, timestamp: i64 },
}

#[derive(Clone)]
pub struct EventBus {
    pub tx: broadcast::Sender<CrawlEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }
    pub fn emit(&self, event: CrawlEvent) {
        let _ = self.tx.send(event);
    }
}
```

- [ ] **Step 2: AppState 添加 event_bus 字段**

```rust
pub struct AppState {
    pub pool: SqlitePool,
    pub config: AppConfig,
    pub event_bus: EventBus,
}
```

初始化：`let event_bus = EventBus::new(256);`

- [ ] **Step 3: scheduler 在状态/任务/日志变化时 emit**

定位 scheduler 中调用 `update_task_status`、`insert_crawl_log`、状态聚合的位置，在调用后 emit。示例：

```rust
// 任务状态变化后（task: &CrawlTaskRecord 已加载）
event_bus.emit(CrawlEvent::TaskUpdate {
    task: serde_json::to_value(task)?,
});

// 状态聚合后（running/current_page/books_found/books_downloaded/books_failed/message 已有值）
event_bus.emit(CrawlEvent::Status {
    running,
    current_page,
    books_found,
    books_downloaded,
    books_failed,
    message: message.clone(),
});

// 日志写入后（id/level/message/timestamp 从 insert_crawl_log 返回）
event_bus.emit(CrawlEvent::Log {
    id, level, message, timestamp,
});
```

具体字段名需与 scheduler 中实际变量名对齐。若变量名不同，按实际名称替换。

- [ ] **Step 4: src/web/crawl.rs 添加 SSE handler（含 Last-Event-ID 补发）**

```rust
use axum::response::sse::{Event, Sse, KeepAlive};
use axum::http::HeaderMap;
use futures::stream::{Stream, StreamExt};
use tokio_stream::wrappers::BroadcastStream;
use std::convert::Infallible;

pub async fn crawl_stream(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // 客户端重连时通过 Last-Event-ID 头携带最后接收的日志 ID
    let last_log_id: i64 = headers
        .get("Last-Event-ID")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // 补发遗漏的日志事件（从 SQLite 查询 id > last_log_id 的记录）
    let missed_logs = crud::list_logs_after(&state.pool, last_log_id, 200).await.unwrap_or_default();

    // 重连后立即重发 task:full（任务事件不补发，直接全量）
    let task_count = crud::count_all_tasks(&state.pool).await.unwrap_or(0);
    let initial_tasks = if task_count > 0 {
        crud::list_all_tasks(&state.pool, 1000).await.unwrap_or_default()
    } else {
        vec![]
    };

    let rx = state.event_bus.tx.subscribe();

    // 先推送补发的日志和任务全量，再订阅实时事件
    let initial_stream = futures::stream::iter(missed_logs.into_iter().map(|log| {
        let json = serde_json::to_string(&CrawlEvent::Log {
            id: log.id,
            level: log.level,
            message: log.message,
            timestamp: log.created_at,
        }).unwrap_or_default();
        Ok::<_, Infallible>(Event::default().event("log").id(log.id.to_string()).data(json))
    }).chain(std::iter::once({
        let tasks_json = serde_json::to_string(&CrawlEvent::TaskFull {
            tasks: initial_tasks.into_iter().map(|t| serde_json::to_value(&t).unwrap_or_default()).collect(),
        }).unwrap_or_default();
        Ok::<_, Infallible>(Event::default().event("task:full").data(tasks_json))
    })));

    let live_stream = BroadcastStream::new(rx).filter_map(|res| {
        let event = res.ok()?;
        let json = serde_json::to_string(&event).ok()?;
        let event_type = match &event {
            CrawlEvent::Status { .. } => "status",
            CrawlEvent::TaskFull { .. } => "task:full",
            CrawlEvent::TaskUpdate { .. } => "task:update",
            CrawlEvent::Log { id, .. } => {
                return Some(Ok(Event::default().event("log").id(id.to_string()).data(json)));
            }
        };
        Some(Ok(Event::default().event(event_type).data(json)))
    });

    let combined = initial_stream.chain(live_stream);

    Sse::new(combined).keep_alive(KeepAlive::new()
        .interval(std::time::Duration::from_secs(15))
        .text("keep-alive"))
}
```

需在 `crud` 中新增两个函数（已在 Task 6 Step 3 列出）：
- `list_logs_after(pool, after_id, limit) -> Vec<CrawlLogRecord>`
- `count_all_tasks(pool) -> i64`
- `list_all_tasks(pool, limit) -> Vec<CrawlTaskRecord>`

- [ ] **Step 5: 添加批量端点 retry_failed 和 delete_tasks**

```rust
pub async fn retry_failed(State(state): State<AppState>) -> AppResult<Json<Value>> {
    let failed = crud::list_tasks_by_status(&state.pool, "failed", 1000).await?;
    let mut count = 0;
    for task in failed {
        if crud::reset_task_status(&state.pool, task.website_book_id).await.is_ok() {
            count += 1;
        }
    }
    Ok(ok(json!({ "count": count })))
}

#[derive(Deserialize)]
pub struct DeleteTasksParams { pub status: Option<String> }

pub async fn delete_tasks(
    State(state): State<AppState>,
    Query(params): Query<DeleteTasksParams>,
) -> AppResult<Json<Value>> {
    let status = params.status.unwrap_or_default();
    let count = crud::delete_tasks_by_status(&state.pool, &status).await?;
    Ok(ok(json!({ "count": count })))
}
```

- [ ] **Step 6: src/web/mod.rs 注册新路由**

```rust
.route("/api/crawl/tasks", get(crawl::list_tasks).delete(crawl::delete_tasks))
.route("/api/crawl/retry-failed", post(crawl::retry_failed))
.route("/api/crawl/stream", get(crawl::crawl_stream))
```

- [ ] **Step 7: 编译并测试 SSE**

Run:
```bash
cargo build && cargo run
# 另开终端：
curl -N http://127.0.0.1:3000/api/crawl/stream
```
Expected: 持续接收 SSE 事件，每 15s 一次 keep-alive

- [ ] **Step 8: Commit**

```bash
git add crates/banzhu-spider/src/
git commit -m "feat(api): SSE 流式推送爬虫事件 + 批量重试/删除端点"
```

---

### Task 9: stats 端点（heatmap / timeline / reading-session / reading-goal）

**Files:**
- Modify: `crates/banzhu-spider/Cargo.toml`（添加 `chrono = { version = "0.4", features = ["clock"] }`）
- Modify: `crates/banzhu-spider/src/web/stats.rs`
- Modify: `crates/banzhu-spider/src/web/mod.rs`

- [ ] **Step 1: src/web/stats.rs 添加 5 个 handler**

```rust
use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::error::{ok, AppResult};
use crate::AppState;

#[derive(Deserialize)]
pub struct HeatmapParams { pub year: Option<i32> }

#[derive(Serialize)]
pub struct HeatmapPoint {
    pub date: String,
    pub duration_sec: i64,
    pub chapters_read: i64,
}

pub async fn heatmap(
    State(state): State<AppState>,
    Query(params): Query<HeatmapParams>,
) -> AppResult<Json<Value>> {
    let year = params.year.unwrap_or_else(|| {
        chrono::Local::now().year()
    });
    let rows = crud::heatmap_data(&state.pool, year).await?;
    let points: Vec<HeatmapPoint> = rows
        .into_iter()
        .map(|(date, dur, ch)| HeatmapPoint {
            date, duration_sec: dur, chapters_read: ch,
        })
        .collect();
    Ok(ok(points))
}

#[derive(Deserialize)]
pub struct TimelineParams { pub days: Option<i64> }

pub async fn reading_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineParams>,
) -> AppResult<Json<Value>> {
    let days = params.days.unwrap_or(7);
    let rows = crud::reading_timeline(&state.pool, days).await?;
    let points: Vec<HeatmapPoint> = rows
        .into_iter()
        .map(|(date, dur, ch)| HeatmapPoint {
            date, duration_sec: dur, chapters_read: ch,
        })
        .collect();
    Ok(ok(points))
}

#[derive(Deserialize)]
pub struct ReportSessionBody {
    pub book_id: i64,
    pub chapter_order: i64,
    pub duration_sec: i64,
    pub chapters_read: i64,
    pub started_at: i64,
    pub ended_at: i64,
}

pub async fn report_session(
    State(state): State<AppState>,
    Json(body): Json<ReportSessionBody>,
) -> AppResult<Json<Value>> {
    crud::insert_reading_session(
        &state.pool, body.book_id, body.chapter_order,
        body.duration_sec, body.chapters_read,
        body.started_at, body.ended_at,
    ).await?;
    Ok(ok(json!({ "ok": true })))
}

pub async fn get_reading_goal(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let goal = crud::get_reading_goal(&state.pool).await?;
    Ok(ok(goal))
}

#[derive(Deserialize)]
pub struct UpdateGoalBody {
    pub daily_minutes: i64,
    pub daily_chapters: i64,
}

pub async fn update_reading_goal(
    State(state): State<AppState>,
    Json(body): Json<UpdateGoalBody>,
) -> AppResult<Json<Value>> {
    crud::update_reading_goal(&state.pool, body.daily_minutes, body.daily_chapters).await?;
    let goal = crud::get_reading_goal(&state.pool).await?;
    Ok(ok(goal))
}

/// 今日阅读聚合（供 StatsView 调用）
pub async fn today_reading(
    State(state): State<AppState>,
) -> AppResult<Json<Value>> {
    let (duration, chapters) = crud::sum_today_reading(&state.pool).await?;
    Ok(ok(json!({ "duration_sec": duration, "chapters_read": chapters })))
}

/// 阅读历史（最近 N 本）
#[derive(Deserialize)]
pub struct HistoryParams { pub limit: Option<i64> }

pub async fn reading_history(
    State(state): State<AppState>,
    Query(params): Query<HistoryParams>,
) -> AppResult<Json<Value>> {
    let limit = params.limit.unwrap_or(20);
    let rows = crud::reading_history(&state.pool, limit).await?;
    let items: Vec<Value> = rows
        .into_iter()
        .map(|(book_id, last_read, total_dur, total_ch)| json!({
            book_id, last_read_at: last_read,
            total_duration_sec: total_dur, total_chapters: total_ch,
        }))
        .collect();
    Ok(ok(items))
}
```

每个 handler 返回 `AppResult<Json<Value>>` 用 `ok()` 包装。注意 `today_reading` 和 `reading_history` 是 StatsView 必需的端点，需在 `crud` 中实现对应函数（已在 Task 6 列出）。

- [ ] **Step 2: src/web/mod.rs 注册路由**

```rust
.route("/api/stats/heatmap", get(stats::heatmap))
.route("/api/stats/reading-timeline", get(stats::reading_timeline))
.route("/api/stats/reading-session", post(stats::report_session))
.route("/api/stats/reading-goal", get(stats::get_reading_goal).put(stats::update_reading_goal))
.route("/api/stats/today", get(stats::today_reading))
.route("/api/stats/reading-history", get(stats::reading_history))
```

- [ ] **Step 3: 编译并手动测试 7 个端点**

Run:
```bash
cargo build && cargo run
curl http://127.0.0.1:3000/api/stats/heatmap?year=2026
curl http://127.0.0.1:3000/api/stats/reading-goal
curl -X POST http://127.0.0.1:3000/api/stats/reading-session -H "Content-Type: application/json" -d "{\"book_id\":1,\"chapter_order\":1,\"duration_sec\":60,\"chapters_read\":1,\"started_at\":1700000000,\"ended_at\":1700000060}"
curl -X PUT http://127.0.0.1:3000/api/stats/reading-goal -H "Content-Type: application/json" -d "{\"daily_minutes\":45,\"daily_chapters\":10}"
curl http://127.0.0.1:3000/api/stats/today
curl http://127.0.0.1:3000/api/stats/reading-history?limit=10
```
Expected: 全部返回 `{"code":0,"data":...}`

- [ ] **Step 4: Commit**

```bash
git add crates/banzhu-spider/src/ crates/banzhu-spider/Cargo.toml
git commit -m "feat(api): 新增 stats 端点 (heatmap/timeline/session/goal)"
```

---

## P2：核心视图

### Task 10: HomeView + 无限滚动 + BookCard

**Files:**
- Create: `crates/banzhu-spider/frontend/src/api/books.ts`
- Create: `crates/banzhu-spider/frontend/src/api/categories.ts`
- Create: `crates/banzhu-spider/frontend/src/utils/format.ts`
- Create: `crates/banzhu-spider/frontend/src/components/BookCard.vue`
- Create: `crates/banzhu-spider/frontend/src/composables/useInfiniteScroll.ts`
- Modify: `crates/banzhu-spider/frontend/src/views/HomeView.vue`

- [ ] **Step 1: 创建 src/api/books.ts**

```typescript
import { client } from './client'
import type { BookRecord, ChapterRecord, Paginated } from '@/types/api'

export interface ListBooksParams {
  page?: number
  limit?: number
  category?: string
}

export const booksApi = {
  list: (params: ListBooksParams = {}) => {
    const qs = new URLSearchParams()
    if (params.page) qs.set('page', String(params.page))
    if (params.limit) qs.set('limit', String(params.limit))
    if (params.category) qs.set('category', params.category)
    const query = qs.toString()
    return client.get<Paginated<BookRecord>>(`/api/books${query ? `?${query}` : ''}`)
  },
  get: (id: number) => client.get<BookRecord>(`/api/books/${id}`),
  chapters: (id: number) => client.get<ChapterRecord[]>(`/api/books/${id}/chapters`),
  chapterContent: (bookId: number, chapterOrder: number) =>
    client.get<{ chapter: ChapterRecord; sections: { content: string }[] }>(
      `/api/books/${bookId}/chapters/${chapterOrder}`,
    ),
  delete: (id: number) => client.delete<{ ok: boolean }>(`/api/books/${id}`),
  // 文件下载不走 JSON client，直接触发浏览器下载
  exportBook: (id: number, format: 'txt' | 'epub') => {
    const a = document.createElement('a')
    a.href = `/api/export/${id}?format=${format}`
    a.download = ''
    document.body.appendChild(a)
    a.click()
    document.body.removeChild(a)
  },
}
```

- [ ] **Step 2: 创建 src/api/categories.ts**

```typescript
import { client } from './client'
export interface Category { name: string; count: number }
export const categoriesApi = {
  list: () => client.get<Category[]>('/api/categories'),
}
```

- [ ] **Step 3: 创建 src/utils/format.ts**

实现 `formatWordCount(n)`、`formatDate(ts)`、`formatRelativeTime(ts)`。

- [ ] **Step 4: 创建 src/components/BookCard.vue**

书籍卡片，props: `{ book: BookRecord }`，显示首字封面 + 标题 + 作者 + 字数 + 分类。Tailwind 类实现响应式 grid。

- [ ] **Step 5: 创建 src/composables/useInfiniteScroll.ts**

用 `IntersectionObserver` 实现，导出 `{ loading, hasMore, sentinel }`。当 sentinel 进入视口时调用 `loadMore()`，返回 `false` 表示没有更多。

- [ ] **Step 6: 实现 src/views/HomeView.vue**

包含：
- 分类筛选栏（横向滚动按钮）
- 书籍网格（`grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4`）
- 无限滚动 sentinel
- 切换分类时重置列表

- [ ] **Step 7: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问首页显示分类标签和书籍卡片，下拉滚动加载

- [ ] **Step 8: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): HomeView 无限滚动 + 分类筛选 + BookCard"
```

---

### Task 11: BookDetailView + ChapterList

**Files:**
- Create: `crates/banzhu-spider/frontend/src/api/shelf.ts`
- Create: `crates/banzhu-spider/frontend/src/api/progress.ts`
- Create: `crates/banzhu-spider/frontend/src/components/ChapterList.vue`
- Modify: `crates/banzhu-spider/frontend/src/views/BookDetailView.vue`

- [ ] **Step 1: 创建 src/api/shelf.ts**

```typescript
export type ShelfGroup = 'reading' | 'want' | 'finished'
export const shelfApi = {
  list: (group?) => client.get<BookshelfRecord[]>(`/api/bookshelf${group ? `?group=${group}` : ''}`),
  add: (bookId, group) => client.post('/api/bookshelf', { book_id: bookId, group }),
  move: (bookId, group) => client.put(`/api/bookshelf/${bookId}`, { group }),
  remove: (bookId) => client.delete(`/api/bookshelf/${bookId}`),
}
```

- [ ] **Step 2: 创建 src/api/progress.ts**

```typescript
export const progressApi = {
  get: (bookId) => client.get<ReadingProgressRecord | null>(`/api/progress/${bookId}`),
  update: (bookId, data) => client.put(`/api/progress/${bookId}`, data),
}
```

- [ ] **Step 3: 创建 src/components/ChapterList.vue**

分页章节列表，props: `{ chapters, currentOrder?, pageSize? }`，emit: `select(order)`。每页 100 条，分页按钮。

- [ ] **Step 4: 实现 src/views/BookDetailView.vue**

布局：
- 顶部书籍信息（封面 + 标题 + 作者 + 分类 + 字数 + 简介）
- 操作按钮（开始阅读 / 加入书架 / 导出 TXT / 导出 EPUB / 删除）
- 章节列表（ChapterList 组件）

加载时并行 `Promise.all([booksApi.get, booksApi.chapters, progressApi.get, shelfApi.list])`。

- [ ] **Step 5: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/book/1` 显示书籍信息和章节列表

- [ ] **Step 6: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): BookDetailView + ChapterList 组件"
```

---

### Task 12: SearchView

**Files:**
- Create: `crates/banzhu-spider/frontend/src/api/search.ts`
- Modify: `crates/banzhu-spider/frontend/src/views/SearchView.vue`

- [ ] **Step 1: 创建 src/api/search.ts**

```typescript
import { client } from './client'
import type { BookRecord } from '@/types/api'

export interface SearchResult {
  book: BookRecord
  snippet: string
  matched_field: string
}
export interface SearchResponse {
  items: SearchResult[]
  total: number
}
export interface SearchParams {
  q: string
  field?: 'all' | 'title' | 'author' | 'content'
  page?: number
  limit?: number
  exact?: boolean
}

export const searchApi = {
  search: (params: SearchParams) => {
    const qs = new URLSearchParams()
    qs.set('q', params.q)
    if (params.field) qs.set('field', params.field)
    if (params.page) qs.set('page', String(params.page))
    if (params.limit) qs.set('limit', String(params.limit))
    if (params.exact) qs.set('exact', '1')
    return client.get<SearchResponse>(`/api/search?${qs.toString()}`)
  },
}
```

- [ ] **Step 2: 实现 src/views/SearchView.vue**

包含：
- 搜索表单（输入框 + 字段选择 + 提交按钮）
- 结果列表（BookCard + snippet 高亮）
- 空状态/加载状态
- 监听 `route.query.q` 变化自动搜索

高亮用正则替换 `<mark>$1</mark>`，注意转义 query 中的正则特殊字符。

- [ ] **Step 3: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/search?q=xxx` 显示搜索结果，匹配词高亮

- [ ] **Step 4: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): SearchView 全文搜索 + 高亮匹配"
```

---

## P3：阅读体验

### Task 13: ReaderView + usePagination + reader store

**Files:**
- Create: `crates/banzhu-spider/frontend/src/stores/reader.ts`
- Create: `crates/banzhu-spider/frontend/src/composables/usePagination.ts`
- Modify: `crates/banzhu-spider/frontend/src/views/ReaderView.vue`

- [ ] **Step 1: 创建 src/stores/reader.ts**

```typescript
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
```

- [ ] **Step 2: 创建 src/composables/usePagination.ts**

```typescript
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
    return false  // 已到末页，触发下一章
  }

  function prev(): boolean {
    if (currentPage.value > 0) {
      currentPage.value--
      return true
    }
    return false  // 已到首页，触发上一章
  }

  function goTo(page: number) {
    currentPage.value = Math.max(0, Math.min(page, totalPages.value - 1))
  }

  // 内容变化时重置到首页
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
```

- [ ] **Step 3: 实现 src/views/ReaderView.vue**

布局：
- 顶部栏（汉堡菜单 + 章节标题 + 设置按钮）
- 内容区（应用 reader.settings 的 fontSize/lineHeight + theme 背景色）
- 底部栏（上一页 + 页码指示 + 下一页）
- 侧边抽屉（ChapterList，点击汉堡菜单展开）
- 设置面板（ReaderSettings 组件）

事件：
- 触摸滑动翻页（touchstart/touchend 计算 deltaX）
- 键盘左右箭头翻页
- 章节切换调用 `progressApi.update`

- [ ] **Step 4: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/read/1/1` 显示章节内容，移动端可滑动翻页，桌面端整章显示

- [ ] **Step 5: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): ReaderView 桌面整章 + 移动分页 + 进度恢复"
```

---

### Task 14: ReaderSettings + useChapterCache (IndexedDB)

**Files:**
- Create: `crates/banzhu-spider/frontend/src/components/ReaderSettings.vue`
- Create: `crates/banzhu-spider/frontend/src/composables/useChapterCache.ts`

- [ ] **Step 1: 创建 src/components/ReaderSettings.vue**

底部弹出面板，包含：
- 字号滑块（14-24）
- 行距滑块（1.5-2.5，step 0.1）
- 主题按钮组（paper/sepia/white/dark）
- 翻页方式按钮组（scroll/paginate）
- 完成按钮（关闭面板）

所有改动直接 `reader.update({...})`。

- [ ] **Step 2: 创建 src/composables/useChapterCache.ts**

封装 IndexedDB（库名 `banzhu-reader`，store `chapters`）：

```typescript
const DB_NAME = 'banzhu-reader'
const DB_VERSION = 1
const STORE = 'chapters'

export interface CachedChapter {
  bookId: number
  chapterOrder: number
  title: string
  content: string
  cachedAt: number
}

let dbPromise: Promise<IDBDatabase> | null = null

function openDB(): Promise<IDBDatabase> {
  if (dbPromise) return dbPromise
  dbPromise = new Promise((resolve, reject) => {
    const req = indexedDB.open(DB_NAME, DB_VERSION)
    req.onupgradeneeded = () => {
      const db = req.result
      if (!db.objectStoreNames.contains(STORE)) {
        const store = db.createObjectStore(STORE, {
          keyPath: ['bookId', 'chapterOrder'],
        })
        store.createIndex('by_book', 'bookId', { unique: false })
        store.createIndex('by_cached_at', 'cachedAt', { unique: false })
      }
    }
    req.onsuccess = () => resolve(req.result)
    req.onerror = () => reject(req.error)
  })
  return dbPromise
}

function tx<T>(
  mode: IDBTransactionMode,
  fn: (store: IDBObjectStore) => IDBRequest<T>,
): Promise<T> {
  return openDB().then(
    db =>
      new Promise<T>((resolve, reject) => {
        const t = db.transaction(STORE, mode)
        const req = fn(t.objectStore(STORE))
        req.onsuccess = () => resolve(req.result)
        req.onerror = () => reject(req.error)
      }),
  )
}

export function useChapterCache() {
  async function get(bookId: number, chapterOrder: number): Promise<CachedChapter | undefined> {
    return tx<CachedChapter>('readonly', s =>
      s.get([bookId, chapterOrder]) as IDBRequest<CachedChapter>,
    )
  }

  async function put(chapter: CachedChapter): Promise<void> {
    await tx('readwrite', s => s.put(chapter))
  }

  async function deleteBook(bookId: number): Promise<void> {
    // 用 by_book 索引游标遍历删除该书所有章节
    const db = await openDB()
    await new Promise<void>((resolve, reject) => {
      const t = db.transaction(STORE, 'readwrite')
      const idx = t.objectStore(STORE).index('by_book')
      const cursorReq = idx.openCursor(IDBKeyRange.only(bookId))
      cursorReq.onsuccess = () => {
        const cursor = cursorReq.result
        if (cursor) {
          cursor.delete()
          cursor.continue()
        }
      }
      t.oncomplete = () => resolve()
      t.onerror = () => reject(t.error)
    })
  }

  async function clearAll(): Promise<void> {
    await tx('readwrite', s => s.clear())
  }

  async function getBookCount(bookId: number): Promise<number> {
    const db = await openDB()
    return new Promise<number>((resolve, reject) => {
      const t = db.transaction(STORE, 'readonly')
      const idx = t.objectStore(STORE).index('by_book')
      const countReq = idx.count(IDBKeyRange.only(bookId))
      countReq.onsuccess = () => resolve(countReq.result)
      countReq.onerror = () => reject(countReq.error)
    })
  }

  async function estimateSize(): Promise<{ usage: number; quota: number }> {
    if ('storage' in navigator && 'estimate' in navigator.storage) {
      const est = await navigator.storage.estimate()
      return { usage: est.usage ?? 0, quota: est.quota ?? 0 }
    }
    return { usage: 0, quota: 0 }
  }

  async function refreshSize(): Promise<{ usage: number; quota: number }> {
    return estimateSize()
  }

  return {
    get,
    put,
    deleteBook,
    clearAll,
    getBookCount,
    estimateSize,
    refreshSize,
  }
}
```

- [ ] **Step 3: 构建验证**

Run: `cd crates/banzhu-spider/frontend && pnpm build`
Expected: 成功

- [ ] **Step 4: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): ReaderSettings 面板 + IndexedDB 章节缓存封装"
```

---

### Task 15: readingSession store + 上报

**Files:**
- Create: `crates/banzhu-spider/frontend/src/api/stats.ts`
- Create: `crates/banzhu-spider/frontend/src/stores/readingSession.ts`
- Modify: `crates/banzhu-spider/frontend/src/views/ReaderView.vue`（接入上报）

- [ ] **Step 1: 创建 src/api/stats.ts**

```typescript
import { client } from './client'
import type { ReadingGoalRecord } from '@/types/api'

export interface HeatmapPoint {
  date: string
  duration_sec: number
  chapters_read: number
}
export interface TimelinePoint {
  date: string
  duration_sec: number
  chapters_read: number
}
export interface TodayReading {
  duration_sec: number
  chapters_read: number
}
export interface ReadingHistoryItem {
  book_id: number
  last_read_at: number
  total_duration_sec: number
  total_chapters: number
}
export interface ReportSessionBody {
  book_id: number
  chapter_order: number
  duration_sec: number
  chapters_read: number
  started_at: number
  ended_at: number
}

export const statsApi = {
  heatmap: (year?: number) =>
    client.get<HeatmapPoint[]>(`/api/stats/heatmap${year ? `?year=${year}` : ''}`),
  timeline: (days?: number) =>
    client.get<TimelinePoint[]>(`/api/stats/reading-timeline${days ? `?days=${days}` : ''}`),
  reportSession: (data: ReportSessionBody) =>
    client.post<{ ok: boolean }>('/api/stats/reading-session', data),
  getGoal: () => client.get<ReadingGoalRecord>('/api/stats/reading-goal'),
  updateGoal: (daily_minutes: number, daily_chapters: number) =>
    client.put<ReadingGoalRecord>('/api/stats/reading-goal', { daily_minutes, daily_chapters }),
  today: () => client.get<TodayReading>('/api/stats/today'),
  history: (limit = 20) =>
    client.get<ReadingHistoryItem[]>(`/api/stats/reading-history?limit=${limit}`),
}
```

- [ ] **Step 2: 创建 src/stores/readingSession.ts**

```typescript
import { defineStore } from 'pinia'
import { ref } from 'vue'
import { statsApi, type ReportSessionBody } from '@/api/stats'

const FLUSH_INTERVAL_MS = 30_000

export const useReadingSessionStore = defineStore('readingSession', () => {
  const bookId = ref(0)
  const chapterOrder = ref(0)
  const durationSec = ref(0)
  const chaptersRead = ref(0)
  const startedAt = ref(0)
  const timerId = ref<number | null>(null)
  const visible = ref(true)

  function onVisibility() {
    visible.value = !document.hidden
  }

  async function flush() {
    if (durationSec.value === 0 && chaptersRead.value === 0) return
    const body: ReportSessionBody = {
      book_id: bookId.value,
      chapter_order: chapterOrder.value,
      duration_sec: durationSec.value,
      chapters_read: chaptersRead.value,
      started_at: startedAt.value,
      ended_at: Math.floor(Date.now() / 1000),
    }
    try {
      await statsApi.reportSession(body)
    } catch (e) {
      console.warn('上报阅读会话失败', e)
    }
    durationSec.value = 0
    chaptersRead.value = 0
    startedAt.value = Math.floor(Date.now() / 1000)
  }

  function flushBeacon() {
    if (durationSec.value === 0 && chaptersRead.value === 0) return
    const body: ReportSessionBody = {
      book_id: bookId.value,
      chapter_order: chapterOrder.value,
      duration_sec: durationSec.value,
      chapters_read: chaptersRead.value,
      started_at: startedAt.value,
      ended_at: Math.floor(Date.now() / 1000),
    }
    const blob = new Blob([JSON.stringify(body)], { type: 'application/json' })
    navigator.sendBeacon('/api/stats/reading-session', blob)
    durationSec.value = 0
    chaptersRead.value = 0
  }

  function start(bid: number, order: number) {
    // 切换书籍/章节时先 flush 旧会话
    if (bookId.value !== 0 && (bookId.value !== bid || chapterOrder.value !== order)) {
      flush()
    }
    bookId.value = bid
    chapterOrder.value = order
    startedAt.value = Math.floor(Date.now() / 1000)
    durationSec.value = 0
    chaptersRead.value = 0

    if (timerId.value !== null) clearInterval(timerId.value)
    timerId.value = window.setInterval(() => {
      if (visible.value) durationSec.value += 1
      if (durationSec.value > 0 && durationSec.value % FLUSH_INTERVAL_MS / 1000 === 0) {
        flush()
      }
    }, 1000)

    document.addEventListener('visibilitychange', onVisibility)
    window.addEventListener('beforeunload', flushBeacon)
  }

  function markChapterRead() {
    chaptersRead.value++
  }

  function stop() {
    flush()
    if (timerId.value !== null) {
      clearInterval(timerId.value)
      timerId.value = null
    }
    document.removeEventListener('visibilitychange', onVisibility)
    window.removeEventListener('beforeunload', flushBeacon)
  }

  return { start, stop, markChapterRead, flush }
})
```

- [ ] **Step 3: 在 ReaderView.vue 接入**

在 Task 13 实现的 ReaderView.vue 基础上，新增阅读会话上报：

```vue
<script setup lang="ts">
import { onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useReadingSessionStore } from '@/stores/readingSession'
import { booksApi } from '@/api/books'
import { progressApi } from '@/api/progress'

const route = useRoute()
const session = useReadingSessionStore()

const bookId = Number(route.params.bookId)
const chapterOrder = Number(route.params.chapterOrder)

async function loadChapter(order: number) {
  const data = await booksApi.chapterContent(bookId, order)
  // 渲染逻辑（见 Task 13 已实现）
}

function nextChapter() {
  session.markChapterRead()
  // 调用 usePagination 的 next() 失败时加载下一章
}

onMounted(async () => {
  await loadChapter(chapterOrder)
  session.start(bookId, chapterOrder)
})

onUnmounted(() => {
  session.stop()
})

// 章节切换时上报进度
watch(() => route.params.chapterOrder, async (newOrder) => {
  if (newOrder) {
    await progressApi.update(bookId, { chapter_order: Number(newOrder), page_index: 0 })
  }
})
</script>
```

- [ ] **Step 4: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 阅读页打开后开始计时，每 30s 上报，切换章节立即上报，关闭页面前 sendBeacon 兜底

- [ ] **Step 5: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): readingSession store 阅读时长上报 + sendBeacon 兜底"
```

---

## P4：书架 + 爬虫

### Task 16: ShelfView + shelf store

**Files:**
- Create: `crates/banzhu-spider/frontend/src/stores/shelf.ts`
- Modify: `crates/banzhu-spider/frontend/src/views/ShelfView.vue`

- [ ] **Step 1: 创建 src/stores/shelf.ts**

```typescript
export const useShelfStore = defineStore('shelf', () => {
  const items = ref<{ shelf: BookshelfRecord; book: BookRecord }[]>([])
  const loading = ref(false)

  async function load(group?: ShelfGroup) {
    loading.value = true
    try {
      const shelfList = await shelfApi.list(group)
      const books = await Promise.all(shelfList.map(s => booksApi.get(s.book_id)))
      items.value = shelfList.map((shelf, i) => ({ shelf, book: books[i] }))
    } finally {
      loading.value = false
    }
  }

  async function add(bookId, group) { await shelfApi.add(bookId, group); await load() }
  async function move(bookId, group) { await shelfApi.move(bookId, group); await load() }
  async function remove(bookId) {
    await shelfApi.remove(bookId)
    items.value = items.value.filter(i => i.shelf.book_id !== bookId)
  }

  return { items, loading, load, add, move, remove }
})
```

- [ ] **Step 2: 实现 src/views/ShelfView.vue**

包含：
- 三标签页切换（在读/想读/读完）
- 书籍列表（BookCard + 分组选择器 + 移出按钮 + 删缓存按钮）
- 空状态

删缓存按钮调用 `useChapterCache().deleteBook(bookId)`。

- [ ] **Step 3: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/shelf` 显示书架，可切换分组、移动、删除缓存

- [ ] **Step 4: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): ShelfView + shelf store + 缓存删除"
```

---

### Task 17: useSSE + crawler store + crawl API

**Files:**
- Create: `crates/banzhu-spider/frontend/src/api/crawl.ts`
- Create: `crates/banzhu-spider/frontend/src/composables/useSSE.ts`
- Create: `crates/banzhu-spider/frontend/src/stores/crawler.ts`

- [ ] **Step 1: 创建 src/api/crawl.ts**

定义 `CrawlStatus`、`CrawlTask`、`CrawlLog` 接口和 `crawlApi`：
- `status()` / `tasks(params)` / `logs(limit)` / `manual(url)` / `retry(bookId)` / `retryFailed()` / `deleteByStatus(status)`

- [ ] **Step 2: 创建 src/composables/useSSE.ts**

封装 EventSource：
- `connect()` / `close()`
- `on(event, handler)` 注册事件监听
- 自动重连（3s 间隔，连续失败 3 次提示）
- onUnmounted 自动 close
- 导出 `{ connected, error, connect, on, close }`

- [ ] **Step 3: 创建 src/stores/crawler.ts**

```typescript
export const useCrawlerStore = defineStore('crawler', () => {
  const status = ref<CrawlStatus | null>(null)
  const tasks = ref<Map<number, CrawlTask>>(new Map())
  const logs = ref<CrawlLog[]>([])

  const statusCount = computed(() => { /* 聚合各状态计数 */ })
  const sortedTasks = computed(() => { /* 按 failed > running > pending > success > skipped 排序 */ })

  function patchStatus(s) { status.value = s }
  function setTasks(items) { tasks.value = new Map(items.map(t => [t.website_book_id, t])) }
  function patchTask(task) {
    tasks.value.set(task.website_book_id, task)
    tasks.value = new Map(tasks.value)  // 触发响应式
  }
  function appendLog(log) {
    logs.value.push(log)
    if (logs.value.length > 200) logs.value = logs.value.slice(-200)
  }
  function setLogs(items) { logs.value = items.slice(-200) }

  return { status, tasks, logs, statusCount, sortedTasks, patchStatus, setTasks, patchTask, appendLog, setLogs }
})
```

- [ ] **Step 4: 构建验证**

Run: `cd crates/banzhu-spider/frontend && pnpm build`
Expected: 成功

- [ ] **Step 5: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): useSSE + crawler store + crawl API"
```

---

### Task 18: CrawlerView + TaskCard + 分组折叠

**Files:**
- Create: `crates/banzhu-spider/frontend/src/components/TaskCard.vue`
- Modify: `crates/banzhu-spider/frontend/src/views/CrawlerView.vue`

- [ ] **Step 1: 创建 src/components/TaskCard.vue**

任务卡片，props: `{ task: CrawlTask }`，emit: `retry(bookId)`。显示：标题（RouterLink to `/book/:book_id`）、状态徽章、进度条、章节进度、错误消息、book_id、时间、重试按钮。

- [ ] **Step 2: 实现 src/views/CrawlerView.vue**

布局：
- **顶部聚合卡片**：5 个数字卡片（状态/运行中/失败/成功/总进度）
- **手动爬取表单**：URL 输入框 + 提交按钮
- **任务列表**：
  - 工具栏：搜索框 + "重试所有失败"按钮 + "清空已完成"按钮
  - SSE 连接状态提示
  - 分组折叠（failed/running 默认展开，其他折叠）
  - 每组内 `v-for` 渲染 TaskCard（任务数 > 100 时改用 `@tanstack/vue-virtual`）
- **日志面板**：固定高度 + 滚动 + 按级别着色

onMounted：
```typescript
const [status, tasks, logs] = await Promise.all([
  crawlApi.status(),
  crawlApi.tasks({ page: 1, limit: 1000 }),
  crawlApi.logs(200),
])
store.patchStatus(status)
store.setTasks(tasks.items)
store.setLogs(logs)

on('status', (data) => store.patchStatus(data))
on('task:full', (data) => store.setTasks(data.tasks))
on('task:update', (data) => store.patchTask(data.task))
on('log', (data) => store.appendLog(data))
connect()
```

- [ ] **Step 3: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/crawler` 显示状态/任务/日志，SSE 实时推送更新

- [ ] **Step 4: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): CrawlerView SSE 订阅 + 分组折叠 + 任务卡片"
```

---

### Task 19: SettingsView

**Files:**
- Modify: `crates/banzhu-spider/frontend/src/views/SettingsView.vue`

- [ ] **Step 1: 实现 src/views/SettingsView.vue**

三个 section：
1. **主题**：浅色/暗黑按钮
2. **离线缓存**：
   - 显示 `cacheSize / cacheQuota` 进度条
   - 配额 > 80% 时橙色提示
   - "清除全部缓存"按钮（调用 `cache.clearAll()`）
3. **关于**：版本号

onMounted 调用 `cache.estimateSize()` 获取缓存大小。

- [ ] **Step 2: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/settings` 显示主题切换、缓存大小、清除按钮

- [ ] **Step 3: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): SettingsView 主题切换 + 缓存管理"
```

---

## P5：统计 + 目标

### Task 20: HeatmapCalendar 组件

**Files:**
- Create: `crates/banzhu-spider/frontend/src/components/HeatmapCalendar.vue`

- [ ] **Step 1: 实现 HeatmapCalendar.vue**

GitHub Contributions 风格 365 天热力图：
- 53 列 × 7 行 SVG/HTML 网格
- 5 档颜色梯度（0 / 1-15 / 16-30 / 31-60 / >60 分钟）
- 浅色/暗黑模式不同色板（浅色 `#ebedf0 → #216e39`，暗黑 `#161b22 → #39d353`）
- 月份标签（1月-12月）
- 星期标签（日一二三四五六）
- hover 显示 tooltip（日期 + 时长 + 章节数）
- 图例（少 → 多）

props: `{ data: HeatmapPoint[], year: number }`，根据 year 生成年内所有日期的格子，data 中无数据的日期 level=0。

- [ ] **Step 2: 构建验证**

Run: `cd crates/banzhu-spider/frontend && pnpm build`
Expected: 成功

- [ ] **Step 3: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): HeatmapCalendar GitHub 风格热力图组件"
```

---

### Task 21: StatsView 整合 + reading-goal 设置 + 缓存管理入口

**Files:**
- Modify: `crates/banzhu-spider/frontend/src/views/StatsView.vue`

- [ ] **Step 1: 实现 src/views/StatsView.vue**

5 个 section（自上而下）：

1. **今日进度**：两个 StatCard（分钟目标 X/Y、章节目标 X/Y），进度环用 SVG circle
2. **阅读热力图**：HeatmapCalendar 组件，年份切换按钮
3. **最近 7 天明细**：纯 SVG 柱状图（每天一根柱子，高度=duration）
4. **阅读历史 + 缓存管理**（spec 第 4 处缓存管理入口）：最近 20 本书，每行显示：
   - 书名（点击跳转 `/book/:id`）
   - 累计时长（人类可读格式：`2h 15m`）
   - 章节数
   - 上次阅读时间（相对时间：`3 小时前`）
   - 继续阅读按钮（跳转 `/read/:bookId/:lastChapter`）
   - **缓存大小**（通过 `useChapterCache().estimateSize(bookId)` 获取，格式化为 KB/MB）
   - **删除缓存按钮**（仅当缓存大小 > 0 时显示，点击调用 `confirmDialog` 确认后 `useChapterCache().deleteBook(bookId)`，成功后 toast 提示并刷新缓存大小）
5. **设置阅读目标**：两个数字输入框（每日分钟数 / 每日章节数）+ 保存按钮

模板关键结构（阅读历史 section）：

```vue
<section class="mt-8">
  <h2 class="text-xl font-bold mb-4">阅读历史</h2>
  <ul class="divide-y dark:divide-gray-700">
    <li v-for="item in history" :key="item.book_id" class="py-3 flex items-center gap-3">
      <RouterLink :to="`/book/${item.book_id}`" class="flex-1 truncate hover:text-blue-500">
        {{ item.title }}
      </RouterLink>
      <span class="text-sm text-gray-500">{{ formatDuration(item.total_duration_sec) }}</span>
      <span class="text-sm text-gray-500">{{ item.chapters_read }} 章</span>
      <span class="text-sm text-gray-500">{{ formatRelative(item.last_read_at) }}</span>
      <RouterLink :to="`/read/${item.book_id}/${item.last_chapter_order}`"
        class="px-2 py-1 text-sm text-blue-500 hover:underline">继续</RouterLink>
      <span v-if="cacheSizes[item.book_id]" class="text-xs text-gray-400">
        {{ formatBytes(cacheSizes[item.book_id]) }}
      </span>
      <button v-if="cacheSizes[item.book_id]"
        @click="onDeleteCache(item.book_id, item.title)"
        class="px-2 py-1 text-xs text-red-500 hover:underline">删缓存</button>
    </li>
  </ul>
</section>
```

`<script setup lang="ts">` 关键逻辑：

```typescript
import { ref, onMounted } from 'vue'
import { statsApi } from '@/api/stats'
import { useChapterCache } from '@/composables/useChapterCache'
import { useToast } from '@/composables/useToast'
import { useConfirm } from '@/composables/useConfirm'
import HeatmapCalendar from '@/components/HeatmapCalendar.vue'
import StatCard from '@/components/StatCard.vue'

const cache = useChapterCache()
const toast = useToast()
const { confirm } = useConfirm()

const history = ref<Array<{ book_id: number; title: string; total_duration_sec: number; chapters_read: number; last_read_at: number; last_chapter_order: number }>>([])
const cacheSizes = ref<Record<number, number>>({})

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / 1024 / 1024).toFixed(2)} MB`
}

function formatDuration(sec: number): string {
  if (sec < 60) return `${sec}s`
  const m = Math.floor(sec / 60)
  if (m < 60) return `${m}m`
  return `${Math.floor(m / 60)}h ${m % 60}m`
}

function formatRelative(ts: number): string {
  const diff = Math.floor(Date.now() / 1000 - ts)
  if (diff < 60) return '刚刚'
  if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`
  if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`
  return `${Math.floor(diff / 86400)} 天前`
}

async function refreshCacheSizes() {
  for (const item of history.value) {
    const size = await cache.estimateSize(item.book_id).catch(() => 0)
    cacheSizes.value[item.book_id] = size
  }
}

async function onDeleteCache(bookId: number, title: string) {
  const ok = await confirm(`确定删除《${title}》的章节缓存？`, '删除缓存')
  if (!ok) return
  await cache.deleteBook(bookId)
  cacheSizes.value[bookId] = 0
  toast.success(`已删除《${title}》的缓存`)
}

onMounted(async () => {
  // 并行拉取所有数据
  const [heatmapData, timelineData, goal, todayData, historyData] = await Promise.all([
    statsApi.heatmap(new Date().getFullYear()),
    statsApi.timeline(7),
    statsApi.getGoal(),
    statsApi.today(),
    statsApi.history(20),
  ])
  history.value = historyData
  await refreshCacheSizes()
})
```

- [ ] **Step 2: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/stats` 显示热力图、柱状图、历史（含缓存大小和删除按钮）、目标设置；点击删除缓存弹出确认框，确认后缓存大小归零

- [ ] **Step 3: Commit**

```bash
git add crates/banzhu-spider/frontend/src/
git commit -m "feat(frontend): StatsView 整合热力图 + 柱状图 + 目标设置 + 缓存管理入口"
```

---

## P6：PWA

### Task 22: vite-plugin-pwa 配置 + manifest

**Files:**
- Modify: `crates/banzhu-spider/frontend/vite.config.ts`
- Create: `crates/banzhu-spider/frontend/public/manifest.webmanifest`
- Create: `crates/banzhu-spider/frontend/public/icons/`（PWA 图标 192x192、512x512）
- Create: `crates/banzhu-spider/frontend/src/composables/usePWA.ts`
- Modify: `crates/banzhu-spider/frontend/src/main.ts`

- [ ] **Step 1: 安装 vite-plugin-pwa**

已在 Task 1 的 package.json 中加入。运行 `pnpm install` 确认。

- [ ] **Step 2: vite.config.ts 接入插件**

替换 Task 1 中创建的 `vite.config.ts` 为以下完整内容（合并原有 vue 插件、alias、proxy、build 配置）：

```typescript
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { VitePWA } from 'vite-plugin-pwa'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [
    vue(),
    VitePWA({
      registerType: 'autoUpdate',
      manifest: {
        name: '版主网阅读',
        short_name: '版主网',
        theme_color: '#3b82f6',
        background_color: '#ffffff',
        display: 'standalone',
        icons: [
          { src: '/icons/192.png', sizes: '192x192', type: 'image/png' },
          { src: '/icons/512.png', sizes: '512x512', type: 'image/png' },
        ],
      },
      workbox: {
        runtimeCaching: [
          {
            urlPattern: /\/api\/books\/\d+\/chapters\/\d+$/,
            handler: 'NetworkFirst',
            options: {
              cacheName: 'chapters-cache',
              expiration: { maxEntries: 1000 },
            },
          },
        ],
      },
    }),
  ],
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  server: {
    port: 5173,
    proxy: { '/api': { target: 'http://127.0.0.1:3000', changeOrigin: true } },
  },
  build: { outDir: 'dist', emptyOutDir: true },
})
```

- [ ] **Step 3: 创建 PWA 图标**

生成 192x192 和 512x512 的 PNG 图标（可用 SVG 转 PNG，或在线工具）。放入 `public/icons/`。

- [ ] **Step 4: 创建 src/composables/usePWA.ts**

```typescript
import { ref } from 'vue'
import { registerSW } from 'virtual:pwa-register'

export function usePWA() {
  const needRefresh = ref(false)
  const offlineReady = ref(false)

  const updateSW = registerSW({
    onNeedRefresh() { needRefresh.value = true },
    onOfflineReady() { offlineReady.value = true },
  })

  function update() {
    updateSW(true)
  }

  return { needRefresh, offlineReady, update }
}
```

- [ ] **Step 5: main.ts 注册 SW**

```typescript
import { registerSW } from 'virtual:pwa-register'
registerSW({ immediate: true })
```

- [ ] **Step 6: 在 App.vue 显示更新提示**

```vue
<script setup lang="ts">
import { usePWA } from '@/composables/usePWA'
const { needRefresh, update } = usePWA()
</script>

<template>
  <AppHeader />
  <RouterView />
  <div v-if="needRefresh" class="fixed bottom-4 right-4 p-3 bg-blue-500 text-white rounded shadow">
    发现新版本，<button @click="update" class="underline">点击刷新</button>
  </div>
</template>
```

- [ ] **Step 7: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build`
Expected: `dist/` 包含 `sw.js`、`manifest.webmanifest`、`registerSW.js`

- [ ] **Step 8: Commit**

```bash
git add crates/banzhu-spider/frontend/
git commit -m "feat(frontend): 接入 vite-plugin-pwa + manifest + 更新提示"
```

---

### Task 23: SW 章节缓存拦截 + 预加载

**Files:**
- Modify: `crates/banzhu-spider/frontend/vite.config.ts`（workbox runtimeCaching）
- Create: `crates/banzhu-spider/frontend/src/composables/useChapterPrefetch.ts`
- Modify: `crates/banzhu-spider/frontend/src/views/ReaderView.vue`（接入预加载）

- [ ] **Step 1: 调整 workbox runtimeCaching**

在 `vite.config.ts` 的 `VitePWA.workbox.runtimeCaching` 中：

```typescript
runtimeCaching: [
  // 应用 shell 由 precache 自动处理
  // 章节内容：NetworkFirst，缓存到 chapters-cache
  {
    urlPattern: /\/api\/books\/\d+\/chapters\/\d+$/,
    handler: 'NetworkFirst',
    options: {
      cacheName: 'chapters-cache',
      expiration: { maxEntries: 5000 }, // 永久缓存，限制上限防止失控
      networkTimeoutSeconds: 10,
    },
  },
  // 书籍详情：NetworkFirst，容错
  {
    urlPattern: /\/api\/books\/\d+$/,
    handler: 'NetworkFirst',
    options: {
      cacheName: 'books-cache',
      expiration: { maxEntries: 200 },
    },
  },
  // 爬虫/统计：NetworkOnly（不缓存）
  // 其他 API：默认不缓存
]
```

注意：spec 中提到用 IndexedDB 而非 SW Cache API。但 Workbox 的缓存管理更成熟（自动过期、配额管理）。这里采用**混合方案**：
- SW Cache API 缓存章节（Workbox 管理，自动处理配额）
- IndexedDB 仍保留 `useChapterCache.ts` 用于按书删除（删除时同时清理 SW Cache）

- [ ] **Step 2: 创建 src/composables/useChapterPrefetch.ts**

```typescript
import { useChapterCache } from './useChapterCache'

export function useChapterPrefetch() {
  const cache = useChapterCache()

  async function prefetch(bookId: number, currentOrder: number, count = 3) {
    if ('requestIdleCallback' in window) {
      (window as any).requestIdleCallback(() => doPrefetch(bookId, currentOrder, count))
    } else {
      setTimeout(() => doPrefetch(bookId, currentOrder, count), 1000)
    }
  }

  async function doPrefetch(bookId: number, currentOrder: number, count: number) {
    for (let i = 1; i <= count; i++) {
      const order = currentOrder + i
      const cached = await cache.get(bookId, order)
      if (cached) continue // 已缓存
      try {
        const res = await fetch(`/api/books/${bookId}/chapters/${order}`)
        if (!res.ok) continue
        const data = await res.json()
        if (data.code !== 0) continue
        await cache.put({
          bookId, chapterOrder: order,
          title: data.data.title,
          content: data.data.content,
          cachedAt: Date.now(),
        })
      } catch (e) {
        // 静默失败
      }
    }
  }

  return { prefetch }
}
```

- [ ] **Step 3: ReaderView 接入预加载**

在 `loadChapter` 完成后调用 `prefetch(bookId, order, 3)`。

- [ ] **Step 4: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 打开章节后，DevTools → Application → Cache Storage 看到 `chapters-cache` 出现后续 3 章缓存

- [ ] **Step 5: Commit**

```bash
git add crates/banzhu-spider/frontend/
git commit -m "feat(frontend): SW 章节缓存 + 预加载下一 3 章"
```

---

### Task 24: 缓存管理 UI（详情页 + 书架页）

**Files:**
- Modify: `crates/banzhu-spider/frontend/src/views/BookDetailView.vue`（显示已缓存章节数 + 删除按钮）
- Modify: `crates/banzhu-spider/frontend/src/composables/useChapterCache.ts`（添加 deleteBookFromSW 函数）

注：ShelfView 已在 Task 16 接入删缓存按钮，SettingsView 已在 Task 19 接入清除全部。本 task 只需补充 BookDetailView。

- [ ] **Step 1: 在 BookDetailView 显示缓存信息**

```vue
<script setup lang="ts">
import { useChapterCache } from '@/composables/useChapterCache'
const cache = useChapterCache()
const cachedCount = ref(0)

async function refreshCacheCount() {
  cachedCount.value = await cache.getBookCount(bookId)
}

async function deleteCache() {
  if (!confirm(`确认删除 ${book.title} 的 ${cachedCount.value} 章缓存？`)) return
  await cache.deleteBook(bookId)
  // 同时清理 SW Cache（通过 caches API）
  const swCache = await caches.open('chapters-cache')
  const keys = await swCache.keys()
  await Promise.all(
    keys
      .filter(req => req.url.includes(`/api/books/${bookId}/chapters/`))
      .map(req => swCache.delete(req))
  )
  await refreshCacheCount()
}

onMounted(async () => {
  await loadData()
  await refreshCacheCount()
})
</script>

<template>
  <!-- 在操作按钮区添加 -->
  <div class="text-xs text-gray-500 mt-2">
    已缓存 {{ cachedCount }} / {{ chapters.length }} 章
    <button v-if="cachedCount > 0" @click="deleteCache" class="ml-2 text-orange-500">删除缓存</button>
  </div>
</template>
```

- [ ] **Step 2: 申请持久化存储**

在 `main.ts` 中：

```typescript
if ('storage' in navigator && 'persist' in navigator.storage) {
  navigator.storage.persist().then((persisted) => {
    if (persisted) console.log('持久化存储已启用')
  })
}
```

- [ ] **Step 3: 构建并测试**

Run: `cd crates/banzhu-spider/frontend && pnpm build && cd .. && cargo run`
Expected: 访问 `/book/1` 显示缓存章节数，删除按钮工作

- [ ] **Step 4: Commit**

```bash
git add crates/banzhu-spider/frontend/
git commit -m "feat(frontend): BookDetailView 缓存管理 UI + 持久化存储"
```

---

## P7：切换 + 清理

### Task 25: 切换 rust-embed 指向 frontend/dist + smoke test

**Files:**
- Verify: `crates/banzhu-spider/src/web/mod.rs`（已在 Task 2 配置）
- Verify: `crates/banzhu-spider/frontend/dist/`（构建产物）
- Test: `crates/banzhu-spider/examples/smoke_test.rs`

- [ ] **Step 1: 完整构建**

Run:
```bash
cd crates/banzhu-spider/frontend && pnpm build
cd .. && cargo build --release
```
Expected: 两者都成功，`frontend/dist/index.html` 存在，binary 大小合理

- [ ] **Step 2: 启动 release 版本验证**

Run:
```bash
cd crates/banzhu-spider && cargo run --release
```
Expected:
- 访问 http://127.0.0.1:3000/ 显示首页
- 路由切换正常（8 个视图）
- 主题切换工作
- API 调用成功（书籍列表、搜索、爬虫状态）

- [ ] **Step 3: 运行 smoke test**

Run:
```bash
cd crates/banzhu-spider && cargo test --example smoke_test
```
Expected: 全部通过

- [ ] **Step 4: 测试 SSE 流**

Run:
```bash
cargo run --release
# 另开终端：
curl -N http://127.0.0.1:3000/api/crawl/stream
```
Expected: 接收到 SSE 事件流

- [ ] **Step 5: 测试 PWA**

在浏览器 DevTools → Application → Service Workers 检查 SW 已注册，Manifest 已加载。

- [ ] **Step 6: Commit（如有修改）**

```bash
git add -A
git commit -m "test: smoke test 全过 + release 构建验证"
```

---

### Task 26: 删除旧 static/ + 文档更新

**Files:**
- Delete: `crates/banzhu-spider/static/`（整个目录）
- Modify: `crates/banzhu-spider/src/web/mod.rs`（移除任何遗留的 ServeDir 引用）
- Modify: `crates/banzhu-spider/Cargo.toml`（移除 `tower-http` 的 `fs` feature，若不再需要）

- [ ] **Step 1: 确认旧 static/ 已无引用**

Run:
```bash
cd f:/project/banzhu-rs
git grep "static/" -- "*.rs"
git grep "ServeDir" -- "*.rs"
```
Expected: 无匹配（或仅注释）

- [ ] **Step 2: 删除 static/ 目录**

使用文件删除工具删除 `crates/banzhu-spider/static/` 整个目录。

- [ ] **Step 3: 清理 Cargo.toml**

移除 `tower-http` 的 `fs` feature（保留 `cors`、`trace` 等仍在用的）：

```toml
# 旧：
tower-http = { version = "0.6", features = ["fs", "cors", "trace"] }
# 新：
tower-http = { version = "0.6", features = ["cors", "trace"] }
```

- [ ] **Step 4: 编译验证**

Run:
```bash
cd crates/banzhu-spider && cargo build --release
```
Expected: 成功，无 warning

- [ ] **Step 5: 最终 smoke test**

Run:
```bash
cargo test --example smoke_test
cargo run --release
```
Expected: 全部通过，浏览器访问正常

- [ ] **Step 6: 更新 README（可选）**

在 `README.md` 添加前端开发说明：
- `cd crates/banzhu-spider/frontend && pnpm install`
- `pnpm dev` 启动前端 dev server
- `pnpm build` 构建前端
- `cargo run` 启动后端（自动嵌入前端产物）

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: 删除旧 vanilla JS static/ 目录 + 清理依赖"
```

---

### Task 27: 前端 Vitest 单元测试

**Files:**
- Create: `crates/banzhu-spider/frontend/vitest.config.ts`
- Create: `crates/banzhu-spider/frontend/src/composables/__tests__/useSSE.test.ts`
- Create: `crates/banzhu-spider/frontend/src/composables/__tests__/usePagination.test.ts`
- Create: `crates/banzhu-spider/frontend/src/composables/__tests__/useChapterCache.test.ts`
- Create: `crates/banzhu-spider/frontend/src/components/__tests__/HeatmapCalendar.test.ts`
- Modify: `crates/banzhu-spider/frontend/package.json`（确认 `test` 脚本已配置）

- [ ] **Step 1: 创建 vitest.config.ts**

```typescript
import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { fileURLToPath, URL } from 'node:url'

export default defineConfig({
  plugins: [vue()],
  test: {
    environment: 'jsdom',
    globals: true,
    include: ['src/**/__tests__/**/*.test.ts'],
  },
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
})
```

- [ ] **Step 2: 编写 useSSE 测试（mock EventSource）**

`src/composables/__tests__/useSSE.test.ts`：

```typescript
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useSSE } from '../useSSE'

class MockEventSource {
  static instances: MockEventSource[] = []
  url: string
  listeners: Record<string, EventListener[]> = {}
  readyState = 0
  constructor(url: string) {
    this.url = url
    MockEventSource.instances.push(this)
  }
  addEventListener(type: string, fn: EventListener) {
    (this.listeners[type] ||= []).push(fn)
  }
  removeEventListener(type: string, fn: EventListener) {
    this.listeners[type] = (this.listeners[type] || []).filter(f => f !== fn)
  }
  close() { this.readyState = 2 }
  emit(type: string, data: unknown) {
    (this.listeners[type] || []).forEach(fn => fn({ data: JSON.stringify(data) } as MessageEvent))
  }
}

describe('useSSE', () => {
  beforeEach(() => {
    MockEventSource.instances = []
    ;(globalThis as unknown as { EventSource: typeof MockEventSource }).EventSource = MockEventSource
  })
  afterEach(() => { vi.useRealTimers() })

  it('订阅后能收到事件并触发回调', async () => {
    const onLog = vi.fn()
    const { subscribe } = useSSE()
    subscribe('/api/crawl/stream', { log: onLog })
    const es = MockEventSource.instances[0]!
    es.emit('log', { id: 1, level: 'INFO', msg: 'test' })
    expect(onLog).toHaveBeenCalledWith({ id: 1, level: 'INFO', msg: 'test' })
  })

  it('unsubscribe 关闭 EventSource', () => {
    const { subscribe, unsubscribe } = useSSE()
    subscribe('/api/crawl/stream', {})
    const es = MockEventSource.instances[0]!
    expect(es.readyState).toBe(0)
    unsubscribe()
    expect(es.readyState).toBe(2)
  })

  it('Last-Event-ID 被记录到内部状态用于重连', () => {
    const { subscribe, getLastEventId } = useSSE()
    subscribe('/api/crawl/stream', { log: () => {} })
    const es = MockEventSource.instances[0]!
    es.emit('log', { id: 42, level: 'INFO', msg: 'msg' })
    expect(getLastEventId()).toBe(42)
  })
})
```

- [ ] **Step 3: 编写 usePagination 测试**

`src/composables/__tests__/usePagination.test.ts`：

```typescript
import { describe, it, expect } from 'vitest'
import { ref } from 'vue'
import { usePagination } from '../usePagination'

const SAMPLE = 'a'.repeat(2000) // 2000 字符

describe('usePagination', () => {
  it('桌面模式（containerWidth >= 768）不分页', () => {
    const content = ref(SAMPLE)
    const width = ref(1024)
    const { isPaginated, totalPages } = usePagination({ content, charsPerPage: 500, containerWidth: width })
    expect(isPaginated.value).toBe(false)
    expect(totalPages.value).toBe(1)
  })

  it('移动模式（containerWidth < 768）按 charsPerPage 分页', () => {
    const content = ref(SAMPLE)
    const width = ref(375)
    const { isPaginated, totalPages, currentPage, next, prev, goTo } = usePagination({
      content, charsPerPage: 500, containerWidth: width,
    })
    expect(isPaginated.value).toBe(true)
    expect(totalPages.value).toBe(4)
    expect(currentPage.value).toBe(1)
    next()
    expect(currentPage.value).toBe(2)
    prev()
    expect(currentPage.value).toBe(1)
    goTo(4)
    expect(currentPage.value).toBe(4)
    goTo(99) // 越界
    expect(currentPage.value).toBe(4)
  })

  it('currentContent 返回当前页切片', () => {
    const content = ref('0123456789')
    const width = ref(375)
    const { currentContent, goTo } = usePagination({ content, charsPerPage: 3, containerWidth: width })
    expect(currentContent.value).toBe('012')
    goTo(2)
    expect(currentContent.value).toBe('345')
    goTo(4)
    expect(currentContent.value).toBe('9')
  })
})
```

- [ ] **Step 4: 编写 useChapterCache 测试（mock IndexedDB）**

`src/composables/__tests__/useChapterCache.test.ts`：

```typescript
import { describe, it, expect, beforeEach } from 'vitest'
import { useChapterCache } from '../useChapterCache'

// 内存版 IndexedDB mock
function createIDBMock() {
  const store = new Map<string, unknown>()
  const db = {
    transaction: () => ({
      objectStore: () => ({
        put: (v: unknown) => { store.set(String((v as { bookId: number; chapterOrder: number }).bookId) + ':' + (v as { chapterOrder: number }).chapterOrder, v); return { onsuccess: null, onerror: null } },
        get: (k: string) => {
          const req = { result: store.get(k) ?? null, onsuccess: null as null | (() => void), onerror: null }
          setTimeout(() => req.onsuccess?.(), 0)
          return req
        },
        openCursor: () => {
          const entries = [...store.entries()]
          let i = 0
          const req = {
            result: null as null | { key: string; value: unknown; continue: () => void },
            onsuccess: null as null | (() => void),
            onerror: null,
          }
          function next() {
            if (i < entries.length) {
              const [key, value] = entries[i]!
              req.result = { key, value, continue: () => { i++; setTimeout(next, 0) } }
            } else {
              req.result = null
            }
            req.onsuccess?.()
          }
          setTimeout(next, 0)
          return req
        },
        count: () => {
          const req = { result: store.size, onsuccess: null as null | (() => void), onerror: null }
          setTimeout(() => req.onsuccess?.(), 0)
          return req
        },
      }),
    }),
  }
  return { db, store }
}

describe('useChapterCache', () => {
  let cache: ReturnType<typeof useChapterCache>
  beforeEach(() => {
    const mock = createIDBMock()
    const openDBMock = async () => mock.db
    ;(globalThis as unknown as { indexedDB: unknown }).indexedDB = { open: () => ({ onsuccess: null, onerror: null, onupgradeneeded: null, result: mock.db }) }
    cache = useChapterCache(openDBMock)
  })

  it('put 后 get 能取回数据', async () => {
    await cache.put({ bookId: 1, chapterOrder: 5, title: '第五章', content: '内容', cachedAt: Date.now() })
    const v = await cache.get(1, 5)
    expect(v).not.toBeNull()
    expect(v!.title).toBe('第五章')
  })

  it('deleteBook 删除该书所有章节', async () => {
    await cache.put({ bookId: 1, chapterOrder: 1, title: '1', content: 'x', cachedAt: 0 })
    await cache.put({ bookId: 1, chapterOrder: 2, title: '2', content: 'x', cachedAt: 0 })
    await cache.put({ bookId: 2, chapterOrder: 1, title: '3', content: 'x', cachedAt: 0 })
    await cache.deleteBook(1)
    expect(await cache.get(1, 1)).toBeNull()
    expect(await cache.get(1, 2)).toBeNull()
    expect(await cache.get(2, 1)).not.toBeNull()
  })

  it('getBookCount 返回章节总数', async () => {
    await cache.put({ bookId: 1, chapterOrder: 1, title: '1', content: 'x', cachedAt: 0 })
    await cache.put({ bookId: 1, chapterOrder: 2, title: '2', content: 'x', cachedAt: 0 })
    expect(await cache.getBookCount(1)).toBe(2)
  })
})
```

- [ ] **Step 5: 编写 HeatmapCalendar 渲染测试**

`src/components/__tests__/HeatmapCalendar.test.ts`：

```typescript
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import HeatmapCalendar from '../HeatmapCalendar.vue'

const DATA = [
  { date: '2026-01-01', duration_sec: 600, chapters_read: 1 },
  { date: '2026-01-02', duration_sec: 3600, chapters_read: 5 },
  { date: '2026-06-15', duration_sec: 120, chapters_read: 0 },
]

describe('HeatmapCalendar', () => {
  it('渲染 53 列 × 7 行 = 371 个格子', () => {
    const wrapper = mount(HeatmapCalendar, { props: { data: DATA, year: 2026 } })
    const cells = wrapper.findAll('[data-test="heatmap-cell"]')
    expect(cells.length).toBe(53 * 7)
  })

  it('有数据的日子根据 duration 选择颜色档位', () => {
    const wrapper = mount(HeatmapCalendar, { props: { data: DATA, year: 2026 } })
    const jan1 = wrapper.find('[data-test="heatmap-cell"][data-date="2026-01-01"]')
    expect(jan1.exists()).toBe(true)
    expect(jan1.classes()).toContain('fill-level-1') // 600s = 10m → 档位 1
    const jan2 = wrapper.find('[data-test="heatmap-cell"][data-date="2026-01-02"]')
    expect(jan2.classes()).toContain('fill-level-4') // 3600s = 60m → 档位 4
  })

  it('无数据的日子为空档位', () => {
    const wrapper = mount(HeatmapCalendar, { props: { data: DATA, year: 2026 } })
    const jan3 = wrapper.find('[data-test="heatmap-cell"][data-date="2026-01-03"]')
    expect(jan3.classes()).toContain('fill-level-0')
  })
})
```

- [ ] **Step 6: 运行测试验证通过**

Run: `cd crates/banzhu-spider/frontend && pnpm test`
Expected: 全部测试通过，0 failures

- [ ] **Step 7: Commit**

```bash
git add crates/banzhu-spider/frontend/vitest.config.ts crates/banzhu-spider/frontend/src/composables/__tests__/ crates/banzhu-spider/frontend/src/components/__tests__/
git commit -m "test(frontend): 新增 Vitest 单测覆盖 useSSE/usePagination/useChapterCache/HeatmapCalendar"
```

---

### Task 28: 后端集成测试（SSE + API 信封）

**Files:**
- Create: `crates/banzhu-spider/tests/sse_test.rs`
- Create: `crates/banzhu-spider/tests/api_test.rs`
- Create: `crates/banzhu-spider/tests/common/mod.rs`

- [ ] **Step 1: 创建 tests/common/mod.rs（测试 fixture）**

```rust
use banzhu_spider::db::init_pool;
use banzhu_spider::state::AppState;
use banzhu_spider::task::scheduler::Scheduler;
use sqlx::{SqlitePool, Row};
use std::sync::Arc;
use tokio::sync::broadcast;

// 内联 schema SQL（与 src/db/mod.rs::init_schema 保持一致，避免引用私有符号）
const SCHEMA_SQL: &str = include_str!("../../src/db/schema.sql");

const EXTRA_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS reading_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    book_id INTEGER NOT NULL,
    chapter_order INTEGER NOT NULL,
    duration_sec INTEGER NOT NULL CHECK(duration_sec > 0),
    chapters_read INTEGER NOT NULL DEFAULT 0,
    started_at INTEGER NOT NULL,
    ended_at INTEGER NOT NULL,
    FOREIGN KEY (book_id) REFERENCES books(id) ON DELETE CASCADE
);
CREATE INDEX IF NOT EXISTS idx_reading_sessions_book ON reading_sessions(book_id);
CREATE INDEX IF NOT EXISTS idx_reading_sessions_started ON reading_sessions(started_at DESC);

CREATE TABLE IF NOT EXISTS reading_goals (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    daily_minutes INTEGER NOT NULL DEFAULT 30 CHECK(daily_minutes >= 0),
    daily_chapters INTEGER NOT NULL DEFAULT 5 CHECK(daily_chapters >= 0),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
INSERT OR IGNORE INTO reading_goals (id) VALUES (1);
"#;

pub async fn setup_state() -> AppState {
    let pool: SqlitePool = init_pool("sqlite::memory:").await.expect("init_pool 失败");
    // 执行 schema 初始化
    sqlx::query(SCHEMA_SQL).execute(&pool).await.expect("schema.sql 初始化失败");
    sqlx::query(EXTRA_SCHEMA_SQL).execute(&pool).await.expect("新增表初始化失败");
    // ALTER TABLE（如尚未添加 last_read_at 字段）
    let cols: Vec<String> = sqlx::query("PRAGMA table_info(reading_progress)")
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|r| r.get::<String, _>("name"))
        .collect();
    if !cols.iter().any(|c| c == "last_read_at") {
        sqlx::query("ALTER TABLE reading_progress ADD COLUMN last_read_at INTEGER NOT NULL DEFAULT 0")
            .execute(&pool)
            .await
            .expect("ALTER reading_progress 失败");
    }
    let (event_tx, _) = broadcast::channel(256);
    let scheduler = Scheduler::new(pool.clone(), event_tx.clone());
    AppState {
        pool,
        event_bus: banzhu_spider::event::EventBus { tx: event_tx },
        scheduler: Arc::new(scheduler),
    }
}

pub async fn spawn_app(state: AppState) -> String {
    let app = banzhu_spider::web::router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    format!("http://{}", addr)
}
```

**注意**：上述代码假设 `src/db/schema.sql` 文件存在。若现有代码中 schema 是直接内联在 Rust 代码中，则改为 `include_str!` 引用对应 Rust 文件，或将所有 `CREATE TABLE` 语句抽出为独立 `schema.sql`（推荐做法，便于测试引用）。Task 6 Step 1 已经把新增表 SQL 内联在代码中，Task 28 实施时可同步重构 `src/db/mod.rs` 让 `init_schema()` 同时读取 `schema.sql`。

- [ ] **Step 2: 编写 tests/api_test.rs（统一响应信封 + 新增端点）**

```rust
mod common;

use reqwest::Client;
use serde_json::Value;

#[tokio::test]
async fn list_books_returns_unified_envelope() {
    let state = common::setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    let res = client.get(format!("{}/api/books", base)).send().await.unwrap();
    assert_eq!(res.status(), 200);
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0, "成功响应 code 应为 0");
    assert!(body["data"].is_array(), "data 应为数组");
}

#[tokio::test]
async fn unknown_book_returns_error_envelope() {
    let state = common::setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    let res = client.get(format!("{}/api/books/999999", base)).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], -1, "失败响应 code 应为 -1");
    assert!(body["msg"].is_string(), "失败应有 msg 字段");
    assert!(body.get("data").is_none(), "失败响应不应有 data 字段");
}

#[tokio::test]
async fn reading_goal_get_and_update() {
    let state = common::setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    // 初始默认值
    let res = client.get(format!("{}/api/stats/reading-goal", base)).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["data"]["daily_minutes"], 30);
    assert_eq!(body["data"]["daily_chapters"], 5);

    // 更新
    let res = client
        .put(format!("{}/api/stats/reading-goal", base))
        .json(&serde_json::json!({ "daily_minutes": 60, "daily_chapters": 10 }))
        .send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0);

    // 再次读取确认
    let res = client.get(format!("{}/api/stats/reading-goal", base)).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["data"]["daily_minutes"], 60);
    assert_eq!(body["data"]["daily_chapters"], 10);
}

#[tokio::test]
async fn report_reading_session_persists() {
    let state = common::setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    // 先插入一本书
    sqlx::query("INSERT INTO books (id, title, author) VALUES (1, '测试书', '作者')")
        .execute(&state.pool)
        .await
        .unwrap();

    let res = client
        .post(format!("{}/api/stats/reading-session", base))
        .json(&serde_json::json!({
            "book_id": 1,
            "chapter_order": 1,
            "duration_sec": 300,
            "chapters_read": 1,
            "started_at": 1700000000,
            "ended_at": 1700000300,
        }))
        .send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert_eq!(body["code"], 0);

    // today 端点应能查到
    let res = client.get(format!("{}/api/stats/today", base)).send().await.unwrap();
    let body: Value = res.json().await.unwrap();
    assert!(body["data"]["duration_sec"].as_i64().unwrap() >= 300);
}
```

- [ ] **Step 3: 编写 tests/sse_test.rs（SSE 流端到端）**

```rust
mod common;

use banzhu_spider::event::CrawlEvent;
use reqwest::Client;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn sse_initial_sends_task_full_when_no_tasks() {
    let state = common::setup_state().await;
    let base = common::spawn_app(state).await;
    let client = Client::new();

    let res = client
        .get(format!("{}/api/crawl/stream", base))
        .header("Accept", "text/event-stream")
        .send().await.unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers().get("content-type").unwrap(), "text/event-stream");

    // 读取前 1024 字节，应该包含 event: task:full
    let bytes = timeout(Duration::from_secs(2), res.bytes()).await.unwrap().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("event: task:full") || text.contains("event:status"), "SSE 应推送初始事件");
}

#[tokio::test]
async fn sse_replay_missed_logs_via_last_event_id() {
    let state = common::setup_state().await;
    let base = common::spawn_app(state.clone()).await;
    let client = Client::new();

    // 先写入 3 条日志到 crawl_logs
    for i in 1..=3 {
        sqlx::query("INSERT INTO crawl_logs (id, task_id, level, msg, ts) VALUES (?, 1, 'INFO', ?, ?)")
            .bind(i)
            .bind(format!("日志 {}", i))
            .bind(i)
            .execute(&state.pool)
            .await
            .unwrap();
    }

    // 带 Last-Event-ID: 1 重连，应补发 id=2, id=3 的日志
    let res = client
        .get(format!("{}/api/crawl/stream", base))
        .header("Last-Event-ID", "1")
        .send().await.unwrap();

    let bytes = timeout(Duration::from_secs(2), res.bytes()).await.unwrap().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("日志 2"), "应补发 id=2 的日志");
    assert!(text.contains("日志 3"), "应补发 id=3 的日志");
    assert!(!text.contains("日志 1"), "不应补发 id<=last_event_id 的日志");
}

#[tokio::test]
async fn sse_broadcasts_live_event() {
    let state = common::setup_state().await;
    let base = common::spawn_app(state.clone()).await;
    let client = Client::new();

    // 启动 SSE 订阅
    let res = client
        .get(format!("{}/api/crawl/stream", base))
        .send().await.unwrap();

    // 在另一个连接中触发 broadcast 事件
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        state.event_bus.tx.send(CrawlEvent::Status {
            running: true,
            pending: 0,
            success: 0,
            failed: 0,
        }).unwrap();
    });

    let bytes = timeout(Duration::from_secs(2), res.bytes()).await.unwrap().unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(text.contains("event: status") || text.contains("running"), "应收到实时 status 事件");
}
```

- [ ] **Step 4: 运行后端测试验证通过**

Run:
```bash
cd crates/banzhu-spider
cargo test --test api_test -- --nocapture
cargo test --test sse_test -- --nocapture
```
Expected: 全部测试通过

- [ ] **Step 5: Commit**

```bash
git add crates/banzhu-spider/tests/
git commit -m "test(backend): 新增 SSE 端到端测试 + API 统一信封测试"
```

---

## 验收清单

实施完成后逐项验证：

- [ ] 8 个视图全部实现：HomeView / BookDetailView / SearchView / ReaderView / ShelfView / CrawlerView / StatsView / SettingsView
- [ ] 100+ 并发爬虫任务下，CrawlerView 滚动流畅（FPS ≥ 50）
- [ ] SSE 推送延迟 ≤ 1s（任务状态变化到 UI 更新）
- [ ] 离线模式可阅读已缓存章节，预加载下一 3 章
- [ ] 单 binary 部署，无外部文件依赖
- [ ] `cargo build --release` + `pnpm build` 全过
- [ ] smoke test 全过
- [ ] PWA 可安装到桌面/手机
- [ ] 阅读会话上报正常（30s 节流 + sendBeacon 兜底）
- [ ] 阅读热力图显示全年数据
- [ ] 阅读目标设置可保存
- [ ] 缓存管理 4 处入口工作（设置页/详情页/书架页/统计页）

---

## 实施风险提示

1. **rust-embed 编译期检查**：若 `frontend/dist/index.html` 不存在，编译会失败。开发时先跑 `pnpm build`。
2. **SSE 代理缓冲**：若部署在 Nginx 后面，需在响应头加 `X-Accel-Buffering: no`。
3. **IndexedDB 配额**：用户大量离线阅读后可能触发配额限制。已在 SettingsView 提供清理入口，并通过 `navigator.storage.persist()` 申请持久化。
4. **ts-rs 类型漂移**：每次 Rust 模型变化后必须 `cargo test` 重新生成 TS 类型，否则前端类型检查会失败。
5. **Vue Router history 模式 404**：Axum SPA fallback 已在 Task 2 配置，确保所有未匹配路径返回 `index.html`。
6. **100+ 任务 SSE 全量推送**：采用 `task:full`（初次）+ `task:update`（增量）策略，避免高频全量推送。
