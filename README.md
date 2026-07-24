# Banzhu Spider

Rust 小说爬虫，自动绕过 Cloudflare Turnstile 验证，支持增量爬取、全文搜索、REST API。

## 架构

```
banzhu-rs/                        ← Cargo 项目根
├── Cargo.toml                    ← 项目依赖与元数据
├── spider.toml                   ← 运行时配置
├── src/
│   ├── cf/                       ← Cloudflare 绕过
│   │   ├── mod.rs                ← CfManager (singleflight cookie 管理)
│   │   └── turnstile.rs          ← CDP 穿透 shadow DOM + 模拟点击
│   ├── task/                     ← 下载任务
│   │   ├── mod.rs                ← 下载流程编排
│   │   ├── parse.rs              ← HTML 结构解析
│   │   └── content.rs            ← 正文提取 + 反爬解密
│   ├── db/                       ← 数据层 (SQLite)
│   │   ├── mod.rs                ← 连接管理
│   │   ├── schema.rs             ← DDL + 迁移
│   │   ├── models.rs             ← 数据模型
│   │   ├── crud.rs               ← 增删改查
│   │   └── fts.rs                ← FTS5 全文索引
│   ├── web/                      ← REST API (Axum)
│   │   ├── mod.rs                ← 路由 + 优雅关闭
│   │   ├── books.rs              ← 书籍接口
│   │   ├── search.rs             ← 搜索接口
│   │   └── crawl.rs              ← 爬取控制
│   ├── banzhuspider.rs           ← 爬虫核心 (wreq Chrome137 指纹)
│   ├── scheduler.rs              ← 定时增量爬取
│   ├── search.rs                 ← 中文分词 + BM25
│   ├── crypto.rs                 ← RC4/AES 解密
│   └── appconfig.rs              ← 用户配置
└── examples/
    ├── smoke_test.rs             ← 冒烟测试
    └── download_book.rs          ← 单本下载示例
```

## 核心技术

| 层级 | 方案 |
|------|------|
| TLS 指纹 | wreq Chrome137 模拟 (JA3/JA4) |
| CF 验证 | stealth Chrome + CDP 穿透 closed shadow DOM + Turnstile 点击 |
| Cookie 管理 | singleflight (RwLock + refresh_lock)，20 分钟 TTL 自动刷新 |
| 反爬对抗 | 图片字体映射 + RC4/AES 解密 + GBK 编码处理 |
| 并发模型 | tokio async + buffer_unordered，章节 8 并发，书籍 4 并发 |
| 存储 | SQLite WAL + FTS5 全文搜索 (BM25) |

## 环境要求

- Rust 1.75+
- Chrome/Chromium (用于 CF 验证)
- Windows / macOS / Linux (需 GUI 环境)

## 快速开始

```bash
# 编译
cargo build

# 启动 API 服务 (端口 3000)
cargo run

# 冒烟测试 (验证 CF 绕过 + 下载链路)
cargo run --example smoke_test

# 下载单本书
cargo run --example download_book
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
cargo run --example smoke_test  # 端到端冒烟测试
```

## License

MIT. 仅供学习研究，请遵守目标站点服务条款。
