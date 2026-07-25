# Banzhu Spider

Rust 小说爬虫，自动绕过 Cloudflare Turnstile 验证，支持增量爬取、全文搜索、REST API。

## 架构

```
banzhu-rs/                        ← Cargo 项目根
├── Cargo.toml                    ← 项目依赖与元数据 (wisp path 依赖)
├── spider.toml                   ← 运行时配置
├── src/
│   ├── spider/                   ← 爬虫模块 (wisp 驱动)
│   │   ├── mod.rs                ← build_spider 组装 (SpiderBuilder + 5 callbacks)
│   │   ├── parse.rs              ← HTML 结构解析 + 反爬解密
│   │   ├── stop.rs               ← EmptyPageTracker (停止条件)
│   │   ├── pipeline.rs           ← BatchItemPipeline (批量写 DB)
│   │   └── callbacks.rs          ← 5 个 callback 工厂函数
│   ├── db/                       ← 数据层 (SQLite)
│   │   ├── mod.rs                ← 连接管理
│   │   ├── schema.rs             ← DDL + 迁移
│   │   ├── models.rs             ← 数据模型
│   │   ├── crud.rs               ← 增删改查 + batch_upsert
│   │   └── fts.rs                ← FTS5 全文索引
│   ├── web/                      ← REST API (Axum)
│   │   ├── mod.rs                ← 路由 + 优雅关闭
│   │   ├── books.rs              ← 书籍接口
│   │   ├── search.rs             ← 搜索接口
│   │   └── crawl.rs              ← 爬取控制
│   ├── scheduler.rs              ← 定时增量爬取 (wisp Engine 驱动)
│   ├── search.rs                 ← 中文分词 + BM25
│   ├── event.rs                  ← EventBus
│   ├── crypto.rs                 ← RC4/AES 解密
│   └── appconfig.rs              ← 用户配置
```

## 核心技术

| 层级 | 方案 |
|------|------|
| TLS 指纹 | wisp HTTP 客户端 (Chrome 指纹) |
| CF 验证 | wisp FetchMode::Auto (HTTP → Dynamic → Stealth 逐请求升级) |
| 反爬对抗 | 图片字体映射 + RC4/AES 解密 + GBK 编码处理 |
| 并发模型 | wisp Engine (tokio async, max_concurrent 可配) |
| 存储 | SQLite WAL + FTS5 全文搜索 (BM25) |

## 环境要求

- Rust 1.75+
- wisp (本地 path 依赖 ../wisp)
- Windows / macOS / Linux

## 快速开始

```bash
# 编译
cargo build

# 启动 API 服务 (端口 3000)
cargo run
```

## 配置

`spider.toml`:

```toml
root_url = "https://www.bz555555555.com"

[cron]
enabled = true
schedule = "0 */6 * * *"   # 每 6 小时增量爬取
pages_limit = 50
```

## API

| 方法 | 路径 | 说明 |
|------|------|------|
| GET | `/api/books?page=&limit=&category=` | 书籍列表 |
| GET | `/api/books/:id` | 书籍详情 |
| GET | `/api/books/:id/chapters` | 章节列表 |
| GET | `/api/books/:id/chapters/:order` | 章节内容 |
| GET | `/api/search?q=&field=&page=` | 全文搜索 |
| GET | `/api/categories` | 分类列表 |
| GET | `/api/stats` | 统计信息 |
| POST | `/api/crawl/trigger` | 手动触发爬取 |
| GET | `/api/crawl/status` | 爬取状态 |

## 测试

```bash
cargo test                  # 单元测试
cargo test --test wisp_engine_integration  # 端到端集成测试
```

## License

MIT. 仅供学习研究，请遵守目标站点服务条款。
