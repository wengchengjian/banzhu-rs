//! 连续空页终止条件。
//!
//! wisp StopContext 不暴露"列表页解析为空"信号，用共享原子计数器实现。

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use wisp::crawl::stop::{StopCondition, StopContext};

/// 连续 max_streak 页空列表则停止派发新请求
#[derive(Clone)]
pub struct EmptyPageTracker {
    streak: Arc<AtomicUsize>,
    max_streak: usize,
}

impl EmptyPageTracker {
    pub fn new(max_streak: usize) -> Self {
        Self {
            streak: Arc::new(AtomicUsize::new(0)),
            max_streak,
        }
    }

    /// 列表页解析为空时调用
    pub fn record_empty(&self) {
        self.streak.fetch_add(1, Ordering::SeqCst);
    }

    /// 列表页解析到非空时调用
    pub fn record_non_empty(&self) {
        self.streak.store(0, Ordering::SeqCst);
    }
}

impl StopCondition for EmptyPageTracker {
    fn should_stop(&self, _ctx: &StopContext) -> bool {
        self.streak.load(Ordering::SeqCst) >= self.max_streak
    }
}
