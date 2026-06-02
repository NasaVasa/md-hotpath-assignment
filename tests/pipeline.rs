// Prices are fixed-point cents in tests, so 100_05 means 100.05.
#![allow(clippy::inconsistent_digit_grouping)]

use md_hotpath_assignment::{MarketDataService, MdEvent, QuoteUpdate};
use tokio::sync::mpsc;

fn update(symbol: &str, seq: u64, bid_px: i64, ask_px: i64) -> QuoteUpdate {
    QuoteUpdate::new(symbol, seq, bid_px, ask_px)
}

#[tokio::test(flavor = "current_thread")]
async fn run_drains_events_before_returning() {
    let service = MarketDataService::new();
    let (tx, rx) = mpsc::channel(8);

    tx.send(MdEvent::Incremental(update("AAPL", 1, 100_00, 100_05)))
        .await
        .unwrap();
    tx.send(MdEvent::Incremental(update("AAPL", 2, 100_01, 100_06)))
        .await
        .unwrap();
    drop(tx);

    service.run(rx).await.unwrap();

    let quote = service
        .get_quote("AAPL")
        .await
        .expect("run() must not return before queued events are applied");
    assert_eq!(quote.seq, 2);
    assert_eq!(quote.bid_px, 100_01);

    let stats = service.stats().await;
    assert_eq!(stats.events_seen, 2);
    assert_eq!(stats.applied, 2);
}

#[tokio::test(flavor = "current_thread")]
async fn run_preserves_per_symbol_sequence_contract() {
    let service = MarketDataService::new();
    let (tx, rx) = mpsc::channel(16);

    tx.send(MdEvent::Incremental(update("AAPL", 1, 100_00, 100_05)))
        .await
        .unwrap();
    tx.send(MdEvent::Incremental(update("AAPL", 3, 103_00, 103_05)))
        .await
        .unwrap();
    tx.send(MdEvent::Snapshot(update("AAPL", 3, 103_00, 103_05)))
        .await
        .unwrap();
    tx.send(MdEvent::Incremental(update("AAPL", 4, 104_00, 104_05)))
        .await
        .unwrap();
    drop(tx);

    service.run(rx).await.unwrap();

    let quote = service.get_quote("AAPL").await.unwrap();
    assert_eq!(quote.seq, 4);
    assert_eq!(quote.bid_px, 104_00);

    let stats = service.stats().await;
    assert_eq!(stats.gaps, 1);
    assert_eq!(stats.snapshots, 1);
}
