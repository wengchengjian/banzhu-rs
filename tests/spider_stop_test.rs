use banzhu_spider::spider::stop::EmptyPageTracker;
use wisp::crawl::stop::{StopCondition, StopContext};
use std::time::Duration;

#[test]
fn test_empty_page_tracker_stops_after_streak() {
    let tracker = EmptyPageTracker::new(3);
    let ctx = StopContext {
        pages: 0, items: 0, errors: 0, in_flight: 0,
        elapsed: Duration::from_secs(0), queue_size: 0,
    };
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
    let ctx = StopContext {
        pages: 0, items: 0, errors: 0, in_flight: 0,
        elapsed: Duration::from_secs(0), queue_size: 0,
    };
    tracker.record_empty();
    tracker.record_empty();
    tracker.record_non_empty();
    tracker.record_empty();
    assert!(!tracker.should_stop(&ctx));
}
