# Banzhu Spider

[中文文档](README_zh.md)

A web scraping tool built with Rust, Python, and JavaScript for educational purposes.

> Note: This project is for Rust web scraping learning purposes only. It demonstrates multi-language interoperability between Python, Rust, and JavaScript.

## Features

- Cloudflare bypass using Python's DrissionPage
- Anti-crawler mechanisms handling:
  - Image-based text extraction
  - Font obfuscation
  - JavaScript deobfuscation
  - AES decryption
- Configurable concurrent downloading
- Progress bar visualization
- Automatic retry mechanism

## Architecture

The project uses a multi-language approach to leverage the strengths of each:
- **Rust**: Core spider logic and concurrent downloads
- **Python**: Cloudflare bypass and browser automation
- **JavaScript**: DOM manipulation and decryption

### Components
- `banzhuspider.rs`: Main spider implementation
- `bypass.rs/py`: Cloudflare bypass logic
- `task.rs`: Download task management
- `error.rs`: Error handling
- `jdom.py`: JavaScript DOM operations

## Dependencies

### Python
- DrissionPage: Browser automation and Cloudflare bypass
- execjs: JavaScript execution

### Node.js
- jsdom: DOM manipulation

### Rust
- tokio: Async runtime
- reqwest: HTTP client
- scraper: HTML parsing
- serde: Serialization
- config: Configuration management
- encoding: Character encoding
- opencv: Image processing

## Setup

1. Install Python dependencies:
```bash
pip install DrissionPage execjs
```

2. Install Node.js dependencies:
```bash
npm install
```

3. Configure spider settings in `spider.toml`:
```toml
root_url = "your_target_url"
max_num = 1000  # Maximum number of items to download
start = 1       # Starting index
```

## Usage

```bash
cargo run
```

## Configuration

The spider can be configured through `spider.toml`:
- `root_url`: Target website URL
- `max_num`: Maximum number of items to process
- `start`: Starting index for processing

## Anti-Crawler Mechanisms

### Image and Font Anti-Crawler
The project uses image recognition technology to handle image-based anti-crawler mechanisms, establishing a mapping between images and text. For font-based anti-crawler mechanisms, it processes through font mapping file analysis.

### AES Decryption
The website's encryption key is visible in the frontend, with the first 16 bits as IV and the last 16 bits as key, enabling decryption using this information.

## Known Limitations

- Limited concurrent processing
- Some content parsing may fail
- No command-line interface yet

## Roadmap

- [ ] Improve concurrent processing
- [ ] Add command-line interface for search and download
- [ ] Better error handling and recovery
- [ ] Enhanced logging system
- [ ] Unit test coverage
- [ ] Documentation improvements

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is for educational purposes only. Please ensure you comply with the target website's terms of service and robots.txt policies.