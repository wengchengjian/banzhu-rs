use banzhu_spider::spider::stop::EmptyPageTracker;
use std::collections::HashMap;
use std::time::Duration;
use wisp::crawl::stop::{StopCondition, StopContext};

fn stop_ctx() -> StopContext {
    StopContext {
        pages: 0,
        items: 0,
        errors: 0,
        in_flight: 0,
        elapsed: Duration::from_secs(0),
        queue_size: 0,
        callback_pages: HashMap::new(),
    }
}

#[test]
fn test_empty_page_tracker_stops_after_streak() {
    let tracker = EmptyPageTracker::new(3);
    let ctx = stop_ctx();
    assert!(!tracker.should_stop(&ctx));
    tracker.record_empty();
    assert!(!tracker.should_stop(&ctx));
    tracker.record_empty();
    assert!(!tracker.should_stop(&ctx));
    tracker.record_empty();
    assert!(tracker.should_stop(&ctx));
}

#[test]
fn test_record_non_empty_resets_streak() {
    let tracker = EmptyPageTracker::new(3);
    let ctx = stop_ctx();
    tracker.record_empty();
    tracker.record_empty();
    tracker.record_non_empty();
    tracker.record_empty();
    assert!(!tracker.should_stop(&ctx));
}
