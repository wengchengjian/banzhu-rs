//! 爬虫事件总线：基于 tokio broadcast channel 的多生产者多消费者广播。
//!
//! Scheduler 作为生产者在状态/任务/日志变化时 emit 事件；
//! SSE handler 作为消费者订阅事件并推送给前端。

use serde::Serialize;
use tokio::sync::broadcast;

/// 爬虫事件类型（SSE 推送给前端）
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CrawlEvent {
    /// 爬虫整体状态变化
    Status {
        running: bool,
        current_page: i64,
        pages_limit: i64,
        books_found: i64,
        books_downloaded: i64,
        books_failed: i64,
        books_skipped: i64,
        message: String,
    },
    /// 任务全量快照（重连时下发）
    TaskFull {
        tasks: Vec<serde_json::Value>,
    },
    /// 单个任务状态更新
    TaskUpdate {
        task: serde_json::Value,
    },
    /// 单条日志
    Log {
        id: i64,
        level: String,
        message: String,
        timestamp: i64,
    },
}

/// 事件总线：包装 broadcast::Sender，Clone 友好（broadcast::Sender 本身 Clone）
#[derive(Clone)]
pub struct EventBus {
    pub tx: broadcast::Sender<CrawlEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _rx) = broadcast::channel(capacity);
        Self { tx }
    }

    /// 发送事件。若无订阅者，发送会被静默丢弃（`_ = send` 忽略错误）。
    pub fn emit(&self, event: CrawlEvent) {
        let _ = self.tx.send(event);
    }
}
