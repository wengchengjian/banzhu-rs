# 版主网 前端技术栈迁移设计规格

日期: 2026-07-20
状态: 待审核
关联: 取代 `2026-07-20-web-frontend-design.md`（vanilla JS 版本）

## 概述

将 banzhu-spider 现有 vanilla JS SPA 前端迁移到 Vue 3 + TypeScript + Vite 技术栈，同时全栈重构：保留 Rust + Axum + SQLite 后端，引入 SSE 替代轮询，新增 PWA 离线阅读和阅读统计与目标功能。前端构建产物通过 rust-embed 嵌入到单 binary 部署。

## 技术决策

| 维度 | 选择 | 理由 |
|------|------|------|
| 前端框架 | Vue 3 + `<script setup>` + TypeScript | 用户偏好，生态成熟，学习曲线平缓 |
| 构建工具 | Vite | Vue 官方推荐，HMR 快 |
| 状态管理 | Pinia | Vue 3 官方推荐 |
| UI/CSS | Tailwind CSS 4 + SFC scoped CSS | 原子 CSS、暗黑模式变体简洁 |
| 路由 | Vue Router (history 模式) | URL 干净，后端 SPA fallback 已支持 |
| 后端 | 保留 Rust + Axum + SQLx + SQLite | 现有代码可复用，性能/类型安全已是前沿 |
| 实时通信 | SSE (Server-Sent Events) | 单向推送场景，浏览器原生支持 |
| 部署 | rust-embed 嵌入单 binary | 单文件部署，无外部依赖 |
| PWA 离线 | IndexedDB + vite-plugin-pwa | 容量大、按书索引删除方便 |
| 迁移路径 | 全量重写 + 一次性切换 | vanilla JS 无组件边界可复用，重写成本低于混合维护 |

## 整体架构

### 模块布局

```
banzhu-rs/
├── crates/banzhu-spider/
│   ├── frontend/              ← 新增：Vue 3 + Vite 工程
│   │   ├── src/
│   │   │   ├── main.ts        ← 入口，挂载 Pinia/Router/PWA
│   │   │   ├── App.vue
│   │   │   ├── router/        ← Vue Router (history 模式)
│   │   │   ├── stores/        ← Pinia: theme/shelf/reader/crawler/stats/readingSession
│   │   │   ├── views/         ← 8 个视图
│   │   │   ├── components/    ← 通用组件
│   │   │   ├── api/           ← 类型安全的 API 客户端
│   │   │   ├── composables/   ← useSSE / usePagination / useReader 等
│   │   │   ├── types/         ← ts-rs 生成的 API 类型
│   │   │   └── assets/styles/main.css
│   │   ├── public/
│   │   │   ├── manifest.webmanifest
│   │   │   └── icons/         ← PWA 图标
│   │   ├── vite.config.ts     ← dev proxy → http://127.0.0.1:3000
│   │   ├── tailwind.config.ts
│   │   ├── tsconfig.json
│   │   ├── package.json
│   │   └── index.html
│   ├── src/
│   │   └── web/
│   │       └── mod.rs         ← 改用 rust-embed 提供静态资源 + SPA fallback
│   ├── Cargo.toml             ← 新增 rust-embed、ts-rs 依赖
│   └── build.rs               ← 编译期检查 frontend/dist/ 存在
└── docs/superpowers/specs/
    └── 2026-07-20-frontend-migration-design.md
```

### 构建/部署流程

**开发**：
- `cd frontend && pnpm dev` → Vite dev server (端口 5173) + 代理 `/api/*` 到 Axum (3000)
- `cargo run` → Axum 后端独立运行
- 前端 Vite HMR，后端 cargo-watch 热重载

**生产构建**：
- `pnpm build` → `frontend/dist/`（含 manifest、SW、入口 HTML、JS/CSS chunks）
- `cargo build --release` → rust-embed 在编译期将 `dist/` 嵌入 binary
- 单 binary 部署，无外部文件依赖

**运行时**：
- Axum 路由：`/api/*` 走 API，`/*` 走 rust-embed 静态资源
- SPA fallback：未匹配的路径返回 `index.html`（Vue Router history 模式必需）
- PWA 资源（`/sw.js`, `/manifest.webmanifest`）由 rust-embed 直接提供

### 关键改动

- **删除** `crates/banzhu-spider/static/` 旧 vanilla JS（迁移完成后）
- **新增** `crates/banzhu-spider/frontend/` Vue 工程
- **改造** `crates/banzhu-spider/src/web/mod.rs` 用 `rust-embed::RustEmbed` 替代 `ServeDir`
- **新增** `rust-embed = "8"`、`ts-rs = "10"` 到 `Cargo.toml`
- **新增** `build.rs`：编译期检查 `frontend/dist/index.html` 存在，给出友好错误

## 前端模块划分

### 视图（views/）—— 8 个 SFC

| 文件 | 路由 | 职责 |
|------|------|------|
| `HomeView.vue` | `/` | 书籍列表 + 分类筛选 + 无限滚动 |
| `BookDetailView.vue` | `/book/:id` | 书籍信息 + 章节列表 + 加入书架/导出/删除 |
| `SearchView.vue` | `/search` | 全文搜索 + 字段筛选 + 高亮匹配 |
| `ReaderView.vue` | `/read/:bookId/:chapterOrder` | 桌面整章 + 移动分页 + 设置面板 |
| `ShelfView.vue` | `/shelf` | 三标签页（在读/想读/读完）+ 进度条 |
| `CrawlerView.vue` | `/crawler` | SSE 订阅状态/任务/日志 + 手动爬取 + 虚拟滚动 |
| `StatsView.vue` | `/stats` | 数字卡片 + 热力图 + 阅读历史 + 目标设置 |
| `SettingsView.vue` | `/settings` | 主题/字号/阅读目标/PWA 缓存清理 |

### 组件（components/）

- `AppHeader.vue` —— 顶栏（站名、搜索、导航、主题切换）
- `BookCard.vue` —— 书籍卡片（首页/搜索/书架复用）
- `ReaderSettings.vue` —— 阅读设置抽屉（字号/行距/主题/翻页方式）
- `ChapterList.vue` —— 章节列表（详情页和阅读页侧栏复用）
- `StatCard.vue` —— 数字卡片
- `HeatmapCalendar.vue` —— GitHub 风格 365 天阅读热力图
- `EmptyState.vue` —— 空状态占位
- `LoadingSpinner.vue` —— 加载占位
- `ToastContainer.vue` —— 全局轻提示（替代 `alert()`）
- `ConfirmDialog.vue` —— 全局确认弹窗（替代 `confirm()`）

### 状态（stores/）—— Pinia

| Store | 持久化 | 内容 |
|-------|--------|------|
| `theme.ts` | localStorage | `light` / `dark` |
| `reader.ts` | localStorage | 字号/行距/主题/翻页方式/当前进度 |
| `shelf.ts` | 否 | 书架缓存（每次进入刷新） |
| `crawler.ts` | 否 | SSE 推送的状态/任务/日志（`Map<number, Task>` 缓存） |
| `stats.ts` | 否 | 统计数据缓存 |
| `readingSession.ts` | 否 | 当前阅读会话时长（前端计时，定时上报） |

### API 客户端（api/）

- `client.ts` —— `fetch` 封装：统一 baseURL、JSON 解析、错误处理、超时（30s）、SSE EventSource 工厂
- `books.ts` —— `listBooks` / `getBook` / `getChapters` / `getChapterContent` / `deleteBook` / `exportBook`
- `search.ts` —— `search`
- `shelf.ts` —— `listShelf` / `addToShelf` / `moveGroup` / `removeFromShelf`
- `progress.ts` —— `getProgress` / `updateProgress`
- `crawl.ts` —— `manualCrawl` / `retryTask` / `retryAllFailed` / `getStatus` / `getTasks` / `getLogs`
- `stats.ts` —— `getStats` / `getStatsDetail` / `getHeatmap` / `getReadingTimeline` / `reportReadingSession` / `getReadingGoal` / `updateReadingGoal`
- 全部基于 `types/api.ts` 中的 TS 类型（由 `ts-rs` 从 Rust 模型自动生成）

### 组合式函数（composables/）

- `useSSE.ts` —— EventSource 封装：自动重连、订阅管理、生命周期清理
- `usePagination.ts` —— 移动端阅读分页逻辑（按字数切页 + 滑动翻页）
- `useReader.ts` —— 阅读进度追踪（章节切换、页码变化、定时上报、会话时长）
- `useInfiniteScroll.ts` —— 首页无限滚动
- `useTheme.ts` —— 主题应用
- `usePWA.ts` —— SW 注册 + 更新提示 + 安装提示
- `useChapterCache.ts` —— IndexedDB 章节缓存读写 + 预加载

### 路由（router/index.ts）

- history 模式
- 路由守卫：记录上一个路由（返回按钮）、阅读页进入时恢复进度
- 滚动行为：路由切换时滚动到顶部（阅读页除外，恢复到上次位置）

## CrawlerView 专门设计（针对 100+ 并发任务）

### 后端 SSE 事件分片推送

| 事件 | 触发 | payload |
|------|------|---------|
| `status` | 状态变化时 | 全量状态对象（运行/空闲/计数） |
| `task:full` | 初次订阅时 | 当前所有任务数组（一次性） |
| `task:update` | 单任务状态变化 | 单个任务对象（增量） |
| `log` | 写入 crawl_logs 时 | 单条日志 |

### 前端视觉分层

1. **顶部聚合卡片**（始终展示）
   - 4 个数字徽章：运行中 / 待执行 / 失败 / 成功
   - 总进度条：已完成 / 总数
   - 一眼掌握全局

2. **任务列表：虚拟滚动 + 分组折叠**
   - 使用 `@tanstack/vue-virtual` 渲染（DOM 节点固定 ~20 个）
   - 分组：失败（默认展开）/ 运行中（默认展开）/ 待执行（折叠）/ 成功（折叠）/ 跳过（折叠）
   - 排序：失败 > 运行中 > 待执行 > 成功 > 跳过；组内按 `started_at` 倒序
   - 任务标题搜索框：实时模糊匹配（book_id / title）

3. **任务卡片**（增量更新）
   - Pinia `crawler` store 用 `Map<number, Task>` 缓存
   - SSE `task:update` 只 patch 单个 card（通过 `:key="task.id"` Vue 复用 DOM）
   - 卡片信息：标题 / book_id / 状态徽章 / 进度条 / 章节进度 / 错误消息 / 重试按钮

4. **底部日志面板**（固定高度 + 虚拟滚动）
   - 默认折叠（点击展开），最多 200 条
   - SSE `log` 事件追加，超过 200 条丢弃最旧
   - 按级别着色：DEBUG 灰 / INFO 白 / WARN 黄 / ERROR 红
   - 自动滚动到底（除非用户手动上滑）

5. **批量操作**
   - "重试所有失败"：调用 `/api/crawl/retry-failed`（新增批量端点）
   - "清空已完成"：调用 `DELETE /api/crawl/tasks?status=success`

### 性能预期

- DOM 节点 < 50（聚合卡片 + 虚拟列表可见项 + 日志面板）
- SSE 流量：初次订阅 1 次全量 + 后续单任务增量（~100 字节/事件）
- 1000 任务场景也能流畅滚动

## 后端调整

### 静态资源托管改造

替换 `src/web/mod.rs` 中的 `ServeDir` 为 rust-embed：

```rust
#[derive(rust_embed::RustEmbed)]
#[folder = "frontend/dist/"]
struct FrontendAsset;

// 路由：/api/* 走 API，其他走 SPA fallback
// 未匹配文件返回 index.html（Vue Router history 模式必需）
```

### SSE 端点新增

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/crawl/stream` | SSE 流，事件类型如上节所列 |
| POST | `/api/crawl/retry-failed` | 批量重试所有失败任务 |
| DELETE | `/api/crawl/tasks?status=success` | 批量删除已完成任务 |

SSE 实现：Axum `Sse<impl Stream<Item = Result<Event, Infallible>>>`，通过 `tokio::sync::broadcast` 订阅 scheduler 内部事件。

scheduler 改造：在 `task/mod.rs` 中增加 `event_tx: broadcast::Sender<CrawlEvent>`，任务状态变更/日志写入时发送事件。SSE handler 订阅该 channel。

SSE 重连策略：客户端 `EventSource` 自带重连，重连后服务端识别 `Last-Event-ID` 头补发遗漏的日志事件（用 SQLite `crawl_logs.id` 作为事件 ID）。任务事件不补发，直接重发 `task:full`。

### API 响应格式统一

现状：API 直接返回数据（无统一信封）。改为：

```typescript
// 成功
{ code: 0, data: T }
// 失败
{ code: -1, msg: string }
```

错误处理中间件：`axum::error_handling` 捕获 `AppError`，转为统一 JSON 响应。

### API 新增/调整

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/stats/detail` | 统计面板数据 |
| GET | `/api/stats/heatmap?year=2026` | 全年阅读热力图数据 |
| GET | `/api/stats/reading-timeline?days=30` | 阅读统计时间线 |
| POST | `/api/stats/reading-session` | 上报阅读会话 |
| GET | `/api/stats/reading-goal` | 获取阅读目标 |
| PUT | `/api/stats/reading-goal` | 设置阅读目标 |

### 数据库 schema 新增

```sql
-- 阅读统计：每次上报一条记录
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

-- 阅读目标（单行表）
CREATE TABLE IF NOT EXISTS reading_goals (
    id INTEGER PRIMARY KEY CHECK(id = 1),
    daily_minutes INTEGER NOT NULL DEFAULT 30 CHECK(daily_minutes >= 0),
    daily_chapters INTEGER NOT NULL DEFAULT 5 CHECK(daily_chapters >= 0),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);
INSERT OR IGNORE INTO reading_goals (id) VALUES (1);
```

`reading_progress` 表新增字段：

```sql
ALTER TABLE reading_progress ADD COLUMN last_read_at INTEGER NOT NULL DEFAULT 0;
```

schema 变更直接用 `CREATE TABLE IF NOT EXISTS` + 启动时执行，不引入独立迁移机制。SQLite `ALTER TABLE ADD COLUMN` 不支持 `IF NOT EXISTS`，启动时尝试执行并忽略已存在错误（或用 `PRAGMA table_info` 检查）。

## 数据流

### 主请求/响应流（标准 API）

```
Vue 组件 → composables → api/* → client.ts → fetch → Axum → SQLx → SQLite
                                 ↓
                          统一响应 {code, data}
                                 ↓
                          Pinia store 缓存 → 组件响应式更新
```

### SSE 推送流（爬虫实时数据）

```
Axum scheduler 状态变化
  ↓ broadcast::Sender<CrawlEvent>
SSE handler (/api/crawl/stream)
  ↓ text/event-stream
浏览器 EventSource
  ↓ useSSE composable
crawler store.patchFromSSE(event)
  ↓ Vue 响应式
CrawlerView 仅更新变化的 task card（:key 复用 DOM）
```

**SSE 事件类型映射**：

| SSE event | store 动作 | UI 影响 |
|-----------|-----------|---------|
| `status` | `crawler.status = payload` | 聚合卡片更新 |
| `task:full` | `crawler.tasks = new Map(payload)` | 任务列表全量渲染 |
| `task:update` | `crawler.tasks.set(id, payload)` | 单卡片 patch |
| `log` | `crawler.logs.push(payload)` + 截断 200 条 | 日志面板追加 |

### 阅读会话上报流

```
ReaderView 挂载 → readingSession.start(bookId, chapterOrder)
                    ↓ 前端 setInterval 每秒 +1
                  duration_sec 累计
ReaderView 卸载 / 章节切换 / 章节完成
  ↓ readingSession.flush()
api.stats.reportReadingSession({...})
  ↓ POST /api/stats/reading-session
SQLite reading_sessions 表
```

**节流策略**：前端累计时长，每 30 秒或章节切换时上报一次，避免高频请求。页面意外关闭时 `navigator.sendBeacon` 兜底上报。

**可见性暂停**：`document.visibilitychange` 事件触发时暂停计时（hidden 暂停、visible 恢复），避免后台标签页虚增时长。

### Pinia 状态流

- `theme.ts`（持久化）：`applyTheme()` → `document.documentElement`
- `reader.ts`（持久化）：字号/行距/主题/模式 → ReaderView 应用样式 + 同步 localStorage
- `shelf.ts`（内存缓存）：进入 ShelfView 时 load()，加入/移除/移动时本地 patch + 后端同步
- `crawler.ts`（SSE 驱动）：进入 CrawlerView 时订阅 SSE，离开时取消订阅（保留最近状态），任务/日志增量更新
- `readingSession.ts`（前端计时）：ReaderView 挂载时 start()，可见性变化时暂停，卸载/章节切换时 flush()

## PWA 缓存方案

### 存储引擎

**IndexedDB**（不用 SW Cache API）：
- 库名 `banzhu-reader`，store `chapters`
- key: `${bookId}:${chapterOrder}`
- value: `{ bookId, chapterOrder, title, content, cachedAt }`
- 索引：`by_book`（按书删除）、`by_cached_at`（清理）

**理由**：IndexedDB 容量更大（可用磁盘 50%+）、按 bookId 索引删除方便、可存结构化元数据。SW 仍拦截章节请求，但内部读写 IndexedDB。

### 预加载策略

```
用户打开第 N 章
  ├─ 主请求：fetch /api/books/:id/chapters/N
  │         ↓ 命中 IndexedDB → 立即返回
  │         ↓ 未命中 → 网络 → 写入 IndexedDB → 返回
  │
  └─ 后台预加载（requestIdleCallback）：
       fetch /api/books/:id/chapters/N+1, N+2, N+3
       ↓ 写入 IndexedDB
       ↓ 失败静默（不阻塞阅读）
```

### SW 拦截规则

| 请求 | 策略 |
|------|------|
| 应用 shell (HTML/JS/CSS) | Cache-First（永久） |
| `/api/books/*/chapters/*` | IndexedDB-First，未命中走网络 + 写入 |
| `/api/books/:id`（详情） | Network-First，离线时读 IndexedDB 元数据 |
| `/api/crawl/*`, `/api/stats/*` | Network-Only（实时性） |
| `/api/shelf`, `/api/progress` | Network-First（容错） |

### 缓存管理入口（4 处）

1. **设置页**：显示总缓存大小 + "清除全部缓存"按钮
2. **书籍详情页**：显示"已缓存 X/Y 章" + "删除缓存"按钮
3. **书架页**：每本书长按/右键菜单 → "删除缓存"
4. **统计页**：阅读历史区块显示每本书缓存大小 + 单本删除

### 配额管理

- 每次 cache 后 `navigator.storage.estimate()` 检查
- 接近配额（>80%）时 Toast 提示用户清理
- **不自动 LRU 淘汰**（用户要求永久缓存）
- 调用 `navigator.storage.persist()` 申请持久化存储，避免浏览器自动清理

### Service Worker 更新

- `vite-plugin-pwa` 的 `autoUpdate` 模式
- 新版本检测 → Toast 提示"发现新版本，点击刷新"
- SW 版本号变更时自动迁移 IndexedDB schema（如需要）

## 阅读统计与目标

### 数据采集

**会话级别**（前端上报）：
- `book_id`, `chapter_order`, `duration_sec`, `chapters_read`, `started_at`, `ended_at`
- 节流：每 30s / 章节切换 / 页面卸载（sendBeacon）时上报
- 可见性变化时暂停计时

**章节级别**（后端记录）：
- 复用现有 `reading_progress` 表（chapter_order, page_index）
- 新增字段：`last_read_at`（最近阅读时间戳）

### 统计聚合

**SQL 视图**（按需查询，不预聚合）：

| 维度 | 查询 |
|------|------|
| 今日阅读时长 | `SELECT SUM(duration_sec) FROM reading_sessions WHERE started_at >= unixepoch('today', 'localtime')` |
| 今日阅读章节 | `SELECT SUM(chapters_read) FROM reading_sessions WHERE started_at >= unixepoch('today', 'localtime')` |
| 7 天时间线 | `SELECT date(started_at, 'unixepoch', 'localtime') as day, SUM(duration_sec) FROM reading_sessions WHERE started_at >= unixepoch('-7 days') GROUP BY day` |
| 单书总时长 | `SELECT SUM(duration_sec) FROM reading_sessions WHERE book_id = ?` |
| 阅读历史 | `SELECT DISTINCT book_id, MAX(started_at) as last_read FROM reading_sessions GROUP BY book_id ORDER BY last_read DESC` |
| 全年热力图 | `SELECT date(started_at, 'unixepoch', 'localtime') as date, SUM(duration_sec), SUM(chapters_read) FROM reading_sessions WHERE started_at >= ? AND started_at < ? GROUP BY date` |

### 目标系统

**单行表 `reading_goals`**：
- `daily_minutes`（默认 30）
- `daily_chapters`（默认 5）

**达成判定**（前端读取后计算）：
- 今日已读时长 ≥ daily_minutes → 时长目标达成
- 今日已读章节 ≥ daily_chapters → 章节目标达成
- UI：进度环（0-100%），达成时显示绿色对勾

### 视图布局

**StatsView 整体布局**：

```
┌─────────────────────────────────────────────┐
│ 今日进度                                    │
│ ┌───────────┐ ┌───────────┐                │
│ │ 25/30 分钟 │ │  3/5 章节  │               │
│ │   83%     │ │   60%     │                │
│ └───────────┘ └───────────┘                │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ 阅读热力图（2026 年，365 天）               │
│ [GitHub 风格 53×7 网格]                     │
│                                             │
│ 少 ■■■■■ 多                                │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ 最近 7 天明细（柱状图，纯 SVG）             │
│ ▆ ▃ ▅ ▇ ▂ ▆ ▄                              │
│ 周一 周二 ... 周日                          │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ 阅读历史（最近 20 本）                      │
│ ┌──────────────────────────────────┐       │
│ │ [书名] 累计 X 分钟 Y 章           │       │
│ │ 上次阅读：3 天前 [继续阅读]       │       │
│ └──────────────────────────────────┘       │
└─────────────────────────────────────────────┘

┌─────────────────────────────────────────────┐
│ 设置阅读目标                                │
│ 每日阅读 [30] 分钟  每日阅读 [5] 章         │
│                              [保存]         │
└─────────────────────────────────────────────┘
```

### 热力图组件实现（`HeatmapCalendar.vue`）

- 53 列 × 7 行网格，每个格子 ~12px × 12px
- 5 档颜色梯度（基于 `duration_sec`）：
  - `0` 分钟 → 灰色 `#ebedf0`
  - `1-15` 分钟 → 浅绿 `#9be9a8`
  - `16-30` 分钟 → 中绿 `#40c463`
  - `31-60` 分钟 → 深绿 `#30a14e`
  - `>60` 分钟 → 最深 `#216e39`
- 暗黑模式：背景 `#161b22`，梯度 `#0e4429 → #006d32 → #26a641 → #39d353`
- hover 显示 tooltip：`2026-07-20，阅读 45 分钟，3 章`
- 点击某天 → 展开当天详情（阅读了哪几本书、各多久）
- 纯 SVG 实现，~371 个 `<rect>`，无第三方图表库

## 错误处理

### 前端错误处理

**API 层（`api/client.ts`）**：

```typescript
// 所有 API 调用走统一封装
async function request<T>(url, options): Promise<T> {
  // 1. 超时控制（30s AbortController）
  // 2. 解析 {code, data, msg} 信封
  // 3. code !== 0 抛出 ApiError(msg, code)
  // 4. 网络错误抛出 NetworkError
  // 5. 5xx 抛出 ServerError
}
```

**组件层**：
- `try/catch` 捕获，通过 `toast.error(msg)` 显示
- 列表加载失败 → 显示 `EmptyState` + 重试按钮
- 关键操作（删除/导出）失败 → 保留原状态 + Toast 提示

**全局错误兜底**：
- `app.config.errorHandler` 捕获未处理异常 → Toast + 控制台
- `window.addEventListener('unhandledrejection')` 兜底
- 路由守卫捕获异步错误 → 跳 500 页面

**SSE 错误**：
- `EventSource` 自带重连，但连续失败 3 次后 `useSSE` 主动断开
- 显示"实时连接断开，3s 后重试"状态条
- 重连成功后清除提示

### 后端错误处理

**统一错误类型**：

```rust
#[derive(thiserror::Error)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("database: {0}")]
    Database(#[from] sqlx::Error),
    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            AppError::NotFound => (404, 1001),
            AppError::BadRequest(_) => (400, 1002),
            AppError::Database(_) => (500, 1003),
            AppError::Internal(_) => (500, 1004),
        };
        Json(json!({code: -1, msg: self.to_string()})).into_response()
    }
}
```

**SSE 错误**：
- 流断开时 `tokio::sync::broadcast::Receiver::recv()` 返回 `Lagged` 错误
- 处理策略：发送 `event: error\ndata: "lagged"` 后重置接收
- 客户端收到 `error` 事件后主动重连（带 `Last-Event-ID`）

**SSE 代理缓冲**：响应头加 `X-Accel-Buffering: no`，避免 Nginx 等代理缓冲 SSE 流。

## 测试策略

### 后端测试（延续现有 `examples/smoke_test.rs` 模式）

- 新增 `tests/sse_test.rs`：SSE 流端到端测试（启 scheduler → 订阅 → 触发任务变更 → 断言事件）
- 新增 `tests/api_test.rs`：统一响应信封测试、新增端点（stats/heatmap、retry-failed）测试
- 不引入 mock，直接用内存 SQLite

### 前端测试

- 单元测试（Vitest）：`useSSE`（mock EventSource）、`usePagination`、`HeatmapCalendar` 渲染、`useChapterCache`（mock IndexedDB）
- 类型对齐：手写 `types/api.ts`，CI 中跑 `cargo test` + `tsc --noEmit` 双向检查
- 不做 E2E（单人项目过度）

### 类型同步方案

- 后端 Rust 模型加 `#[derive(Serialize)]`，新增 `ts-rs = "10"` 依赖
- `cargo test` 时自动生成 `frontend/src/types/api.ts`
- 避免前后端类型漂移

## 实施步骤

| 阶段 | 内容 | 产物 |
|------|------|------|
| **P0：脚手架** | 创建 `frontend/` Vue 工程，接入 Vite + Tailwind + Pinia + Router；接入 rust-embed；dev proxy 通 | 可访问空白页 |
| **P1：后端 SSE** | scheduler 加 `broadcast::Sender`；新增 `/api/crawl/stream`、`/api/crawl/retry-failed`、`DELETE /api/crawl/tasks`；统一响应信封；新增 `reading_sessions`、`reading_goals` 表 | 后端 API 调通 |
| **P2：核心视图** | HomeView + BookDetailView + SearchView（迁移现有 3 视图） | 可浏览/搜索书籍 |
| **P3：阅读体验** | ReaderView（桌面整章 + 移动分页 + 设置）+ 阅读进度上报 + readingSession store | 可阅读 |
| **P4：书架 + 爬虫** | ShelfView + CrawlerView（虚拟滚动 + SSE 订阅）+ 批量操作 | 可管理 |
| **P5：统计 + 目标** | StatsView（热力图 + 柱状图 + 历史）+ reading_goals + heatmap API | 可统计 |
| **P6：PWA** | vite-plugin-pwa + IndexedDB 缓存 + 预加载 + 缓存管理 UI | 可离线 |
| **P7：切换 + 清理** | 切换 rust-embed 指向 `frontend/dist`；删除 `static/` 旧代码；smoke test 全过 | 上线 |

## 验收标准

- 8 个视图全部实现：7 个视图（首页/详情/搜索/阅读/书架/爬虫/统计）功能对齐现有 vanilla JS 版本 + 1 个新增设置页
- 100+ 并发爬虫任务下，CrawlerView 滚动流畅（FPS ≥ 50）
- SSE 推送延迟 ≤ 1s（任务状态变化到 UI 更新）
- 离线模式可阅读已缓存章节，预加载下一 3 章
- 单 binary 部署，无外部文件依赖
- `cargo build --release` + `pnpm build` 全过

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| rust-embed 编译期检查 `frontend/dist` 不存在 | `build.rs` 加友好错误提示，引导先跑 `pnpm build` |
| SSE 在某些代理下被缓冲 | 响应头加 `X-Accel-Buffering: no` |
| IndexedDB 配额满 | 监控 + Toast 提示，提供清理入口 |
| Vue Router history 模式 404 | Axum SPA fallback 在第 3 章覆盖 |
| ts-rs 生成类型与手写 API 不一致 | CI 跑 `cargo test` 后 `git diff` 检查 |
| 100+ 任务 SSE 全量推送流量大 | 采用 `task:full` 一次性 + `task:update` 增量推送 |
