// Prices are fixed-point cents in tests, so 100_05 means 100.05.
#![allow(clippy::inconsistent_digit_grouping)]

use md_hotpath_assignment::{MarketDataService, MdEvent, QuoteUpdate, SymbolHealth};

fn update(symbol: &str, seq: u64, bid_px: i64, ask_px: i64) -> QuoteUpdate {
    QuoteUpdate::new(symbol, seq, bid_px, ask_px).with_qty(10, 11)
}

#[tokio::test]
async fn first_incremental_can_establish_initial_state() {
    let service = MarketDataService::new();

    service
        .apply_event(MdEvent::Incremental(update("AAPL", 42, 100_00, 100_05)))
        .await
        .unwrap();

    let quote = service.get_quote("AAPL").await.expect("quote must exist");
    assert_eq!(quote.seq, 42);
    assert_eq!(quote.bid_px, 100_00);
    assert_eq!(quote.ask_px, 100_05);

    let status = service.symbol_status("AAPL").await;
    assert_eq!(status.health, SymbolHealth::Live);
    assert_eq!(status.last_seq, 42);

    let stats = service.stats().await;
    assert_eq!(stats.events_seen, 1);
    assert_eq!(stats.applied, 1);
    assert_eq!(stats.duplicates, 0);
    assert_eq!(stats.gaps, 0);
}

#[tokio::test]
async fn in_order_updates_are_applied_and_duplicates_are_ignored() {
    let service = MarketDataService::new();

    service
        .apply_event(MdEvent::Incremental(update("AAPL", 1, 100_00, 100_05)))
        .await
        .unwrap();
    service
        .apply_event(MdEvent::Incremental(update("AAPL", 2, 100_01, 100_06)))
        .await
        .unwrap();
    service
        .apply_event(MdEvent::Incremental(update("AAPL", 2, 999_00, 999_05)))
        .await
        .unwrap();
    service
        .apply_event(MdEvent::Incremental(update("AAPL", 1, 888_00, 888_05)))
        .await
        .unwrap();

    let quote = service.get_quote("AAPL").await.expect("quote must exist");
    assert_eq!(quote.seq, 2);
    assert_eq!(quote.bid_px, 100_01);
    assert_eq!(quote.ask_px, 100_06);

    let status = service.symbol_status("AAPL").await;
    assert_eq!(status.health, SymbolHealth::Live);
    assert_eq!(status.last_seq, 2);
    assert_eq!(status.duplicate_count, 2);

    let stats = service.stats().await;
    assert_eq!(stats.events_seen, 4);
    assert_eq!(stats.applied, 2);
    assert_eq!(stats.duplicates, 2);
    assert_eq!(stats.gaps, 0);
}

#[tokio::test]
async fn gap_is_detected_and_out_of_sequence_incremental_is_not_applied() {
    let service = MarketDataService::new();

    service
        .apply_event(MdEvent::Incremental(update("AAPL", 1, 100_00, 100_05)))
        .await
        .unwrap();

    service
        .apply_event(MdEvent::Incremental(update("AAPL", 3, 101_00, 101_05)))
        .await
        .unwrap();

    let quote = service
        .get_quote("AAPL")
        .await
        .expect("old quote must remain");
    assert_eq!(quote.seq, 1, "gap seq=3 must not silently advance state");
    assert_eq!(quote.bid_px, 100_00);

    let status = service.symbol_status("AAPL").await;
    assert_eq!(status.health, SymbolHealth::Stale);
    assert_eq!(
        status.last_seq, 1,
        "last applied seq must remain unchanged after a gap"
    );
    assert_eq!(status.gap_count, 1);

    let stats = service.stats().await;
    assert_eq!(stats.events_seen, 2);
    assert_eq!(stats.applied, 1);
    assert_eq!(stats.gaps, 1);
}

#[tokio::test]
async fn snapshot_recovers_symbol_after_gap_then_next_incremental_can_apply() {
    let service = MarketDataService::new();

    service
        .apply_event(MdEvent::Incremental(update("AAPL", 10, 100_00, 100_05)))
        .await
        .unwrap();
    service
        .apply_event(MdEvent::Incremental(update("AAPL", 12, 102_00, 102_05)))
        .await
        .unwrap();

    assert_eq!(service.get_quote("AAPL").await.unwrap().seq, 10);
    assert_eq!(
        service.symbol_status("AAPL").await.health,
        SymbolHealth::Stale
    );

    service
        .apply_event(MdEvent::Snapshot(update("AAPL", 12, 102_00, 102_05)))
        .await
        .unwrap();

    let recovered = service
        .get_quote("AAPL")
        .await
        .expect("snapshot must recover quote");
    assert_eq!(recovered.seq, 12);
    assert_eq!(recovered.bid_px, 102_00);
    assert_eq!(
        service.symbol_status("AAPL").await.health,
        SymbolHealth::Live
    );

    service
        .apply_event(MdEvent::Incremental(update("AAPL", 13, 103_00, 103_05)))
        .await
        .unwrap();

    let quote = service.get_quote("AAPL").await.unwrap();
    assert_eq!(quote.seq, 13);
    assert_eq!(quote.bid_px, 103_00);

    let stats = service.stats().await;
    assert_eq!(stats.snapshots, 1);
    assert_eq!(stats.gaps, 1);
}

#[tokio::test]
async fn gap_on_one_symbol_does_not_block_other_symbols() {
    let service = MarketDataService::new();

    service
        .apply_event(MdEvent::Incremental(update("AAPL", 1, 100_00, 100_05)))
        .await
        .unwrap();
    service
        .apply_event(MdEvent::Incremental(update("MSFT", 1, 200_00, 200_05)))
        .await
        .unwrap();
    service
        .apply_event(MdEvent::Incremental(update("AAPL", 3, 103_00, 103_05)))
        .await
        .unwrap();
    service
        .apply_event(MdEvent::Incremental(update("MSFT", 2, 201_00, 201_05)))
        .await
        .unwrap();

    assert_eq!(service.get_quote("AAPL").await.unwrap().seq, 1);
    assert_eq!(
        service.symbol_status("AAPL").await.health,
        SymbolHealth::Stale
    );

    let msft = service.get_quote("MSFT").await.unwrap();
    assert_eq!(msft.seq, 2);
    assert_eq!(msft.bid_px, 201_00);
    assert_eq!(
        service.symbol_status("MSFT").await.health,
        SymbolHealth::Live
    );
}
