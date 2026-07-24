# 配置说明

所有配置集中在项目根目录的 `spider.toml` 中。

## 完整配置示例

```toml
# 目标站点地址
root_url = "https://www.bz555555555.com"

[server]
port = 3000                   # API 服务端口

[spider]
max_concurrent_tasks = 16     # 章节并发下载数
retry_attempts = 3            # 请求失败重试次数
retry_delay_ms = 100          # 重试初始退避 (ms)，每次翻倍
request_timeout_secs = 15     # 单次 HTTP 请求超时

[spider.proxy]
enabled = false               # 是否启用代理
url = ""                      # 代理地址，支持 http/https/socks5

[cron]
enabled = true                # 是否启用定时爬取
schedule = "0 */6 * * *"      # cron 表达式
pages_limit = 50              # 单次最多爬取列表页数
book_concurrency = 4          # 书籍并发下载数

[cf_bypass]
cookie_ttl_secs = 1200        # cf_clearance 缓存有效期
chrome_timeout_secs = 120     # 等待 CF 验证通过的最大时间
passive_wait_secs = 2         # 被动等待 JS Challenge 自动解决
click_interval_secs = 2       # Turnstile 点击尝试间隔
headless = false              # Chrome 无头模式
```

## 各节说明

### `root_url`

目标小说站点地址。爬虫所有请求基于此 URL 构建。

### `[server]`

| 键 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `port` | int | 3000 | REST API 监听端口 |

### `[spider]`

| 键 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `max_concurrent_tasks` | int | 16 | 章节内容并发下载数 |
| `retry_attempts` | int | 3 | 请求失败后的重试次数 |
| `retry_delay_ms` | int | 100 | 首次重试等待时间，后续指数递增 |
| `request_timeout_secs` | int | 15 | 单次 HTTP 请求超时（秒） |

### `[spider.proxy]`

| 键 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `enabled` | bool | false | 是否启用代理 |
| `url` | string | "" | 代理地址 |

支持的代理格式：
- HTTP: `http://127.0.0.1:7890`
- HTTPS: `https://proxy.example.com:8080`
- SOCKS5: `socks5://127.0.0.1:1080`

启用代理后，所有 HTTP 请求（包括 CF 验证后的数据请求）都会通过代理发出。Chrome 浏览器的 CF 验证不走此代理（走系统代理）。

### `[cron]`

| 键 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `enabled` | bool | true | 是否启用定时增量爬取 |
| `schedule` | string | "0 */6 * * *" | cron 表达式 |
| `pages_limit` | int | 50 | 单次爬取最大列表页数 |
| `book_concurrency` | int | 4 | 同时下载的书籍数量 |

常用 cron 表达式：
- `0 */6 * * *` — 每 6 小时
- `0 2 * * *` — 每天凌晨 2 点
- `0 */30 * * * *` — 每 30 分钟（6 位，含秒）

### `[cf_bypass]`

| 键 | 类型 | 默认值 | 说明 |
|---|---|---|---|
| `cookie_ttl_secs` | int | 1200 | cf_clearance cookie 缓存时间（秒） |
| `chrome_timeout_secs` | int | 120 | 等待 CF 验证通过的最大时间 |
| `passive_wait_secs` | int | 2 | 页面加载后被动等待时间 |
| `click_interval_secs` | int | 2 | Turnstile 点击重试间隔 |
| `headless` | bool | false | 是否使用 Chrome 无头模式 |

**关于 `headless`：**
- `false`（默认）：弹出 Chrome 窗口，可靠性最高
- `true`：无窗口，适合服务器环境，但可能被 CF 检测到

**关于 `cookie_ttl_secs`：**
Cloudflare 的 cf_clearance 通常 15-30 分钟过期。设为 1200（20 分钟）是安全值。如果频繁遇到 CF 验证，可以降低此值。

## 用户配置（~/.banzhu/config.toml）

除了项目级 `spider.toml`，还有用户级配置存储在 `~/.banzhu/config.toml`：

```toml
save_db_path = "/path/to/banzhu.db"   # 数据库文件路径
root_url = "https://..."               # 覆盖项目级 root_url
```

用户配置优先级高于项目配置。
