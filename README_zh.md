# Banzhu Spider (版主爬虫)

一个使用 Rust、Python 和 JavaScript 构建的网络爬虫工具，用于学习目的。

> 注意：本项目仅用于 Rust 爬虫学习。它展示了 Python、Rust 和 JavaScript 之间的多语言互操作性。

## 功能特点

- 使用 Python 的 DrissionPage 绕过 Cloudflare 防护
- 处理多种反爬虫机制：
  - 基于图像的文本提取
  - 字体混淆处理
  - JavaScript 代码混淆处理
  - AES 解密
- 可配置的并发下载
- 进度条可视化
- 自动重试机制

## 架构设计

项目采用多语言方案，充分利用各语言的优势：
- **Rust**：核心爬虫逻辑和并发下载
- **Python**：Cloudflare 绕过和浏览器自动化
- **JavaScript**：DOM 操作和解密

### 主要组件
- `banzhuspider.rs`：主要爬虫实现
- `bypass.rs/py`：Cloudflare 绕过逻辑
- `task.rs`：下载任务管理
- `error.rs`：错误处理
- `jdom.py`：JavaScript DOM 操作

## 环境要求

- Python 3.8+
- Node.js 14+
- Rust 1.70+
- OpenCV 4.x
- 至少 4GB 内存
- Windows/Linux/MacOS

## 依赖环境

### Python 依赖
- DrissionPage：浏览器自动化和 Cloudflare 绕过
- execjs：JavaScript 执行
- opencv-python：图像处理

### Node.js 依赖
- jsdom：DOM 操作

### Rust 依赖
- tokio：异步运行时
- reqwest：HTTP 客户端
- scraper：HTML 解析
- serde：序列化
- config：配置管理
- encoding：字符编码
- opencv：图像处理
- pyo3：Python 绑定

## 安装设置

1. 安装 Rust（如果未安装）：
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. 安装 Python 依赖：
```bash
pip install DrissionPage execjs opencv-python
```

3. 安装 Node.js 依赖：
```bash
npm install jsdom
```

4. 安装 OpenCV：
- Windows：从 opencv.org 下载并安装
- Linux：`sudo apt-get install libopencv-dev`
- MacOS：`brew install opencv`

5. 在 `spider.toml` 中配置爬虫设置：
```toml
root_url = "目标网站URL"
max_num = 1000  # 最大下载数量
start = 1       # 起始索引
```

## 使用方法

```bash
cargo run
```

## 配置说明

爬虫可以通过 `spider.toml` 进行配置：
- `root_url`：目标网站 URL
- `max_num`：最大处理项目数
- `start`：处理起始索引

## 反爬处理

### 图片和字体反爬
项目使用图像识别技术处理图片反爬，建立图片和文字的对应关系。对于字体反爬，通过解析字体映射文件处理。

### AES 解密
网站的加密密钥在前端可见，前16位为iv，后16位为key，通过这些信息可以进行解密。

## 已知限制

- 并发处理能力有限
- 部分内容解析可能失败
- 暂无命令行界面

## 开发计划

- [√] 提升并发处理能力
- [ ] 添加命令行搜索和下载界面
- [√] 改进错误处理和恢复机制
- [√] 增强日志系统
- [ ] 增加单元测试覆盖率
- [√] 完善文档

## 贡献指南

欢迎提交 Pull Request 来改进项目！

## 许可说明

本项目仅用于教育目的。请确保遵守目标网站的服务条款和 robots.txt 政策。
