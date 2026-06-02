//! Diagnostic stress harness.
//!
//! This test is ignored by default because absolute timing is machine-dependent. Candidates should
//! run it with:
//!
//!     cargo test --test stress -- --ignored --nocapture
//!
//! For local strict mode:
//!
//!     MD_STRESS_STRICT=1 cargo test --test stress -- --ignored --nocapture

use std::time::{Duration, Instant};

use md_hotpath_assignment::{MarketDataService, MdEvent, QuoteUpdate};
use tokio::sync::mpsc;

fn update(symbol_idx: usize, seq: u64) -> QuoteUpdate {
    let symbol = format!("SYM{:05}", symbol_idx);
    let bid = 10_000 + symbol_idx as i64 + seq as i64;
    QuoteUpdate::new(symbol, seq, bid, bid + 1)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[ignore = "diagnostic burst harness; run manually with --ignored --nocapture"]
async fn burst_profile_smoke() {
    const SYMBOLS: usize = 20_000;
    const UPDATES: usize = 200_000;

    let service = MarketDataService::new();
    let (tx, rx) = mpsc::channel::<MdEvent>(8192);

    let runner = {
        let service = service.clone();
        tokio::spawn(async move { service.run(rx).await.unwrap() })
    };

    let mut next_seq = vec![0_u64; SYMBOLS];
    let start = Instant::now();

    // Prime every symbol so the final snapshot-size assertion is deterministic.
    for (symbol_idx, seq) in next_seq.iter_mut().enumerate() {
        *seq += 1;
        tx.send(MdEvent::Incremental(update(symbol_idx, *seq)))
            .await
            .unwrap();
    }

    for i in SYMBOLS..UPDATES {
        // A deliberately skewed profile: every 10th update hits a hot subset of 128 symbols;
        // the rest is spread across the full universe.
        let symbol_idx = if i % 10 == 0 { i % 128 } else { i % SYMBOLS };
        next_seq[symbol_idx] += 1;
        tx.send(MdEvent::Incremental(update(
            symbol_idx,
            next_seq[symbol_idx],
        )))
        .await
        .unwrap();
    }
    drop(tx);

    runner.await.unwrap();
    let elapsed = start.elapsed();
    let stats = service.stats().await;
    let snapshot_size = service.snapshot().await.len();

    println!("updates={UPDATES}");
    println!("elapsed_ms={:.3}", elapsed.as_secs_f64() * 1_000.0);
    println!(
        "throughput_updates_per_sec={:.0}",
        UPDATES as f64 / elapsed.as_secs_f64()
    );
    println!("stats={stats:?}");
    println!("snapshot_size={snapshot_size}");

    assert_eq!(
        stats.events_seen as usize, UPDATES,
        "run must drain all events"
    );
    assert_eq!(stats.gaps, 0, "generated stream is gap-free per symbol");
    assert_eq!(snapshot_size, SYMBOLS, "all symbols should have a quote");

    if std::env::var_os("MD_STRESS_STRICT").is_some() {
        assert!(
            elapsed < Duration::from_millis(1500),
            "strict local smoke budget exceeded: {elapsed:?}; this is not a universal benchmark"
        );
    }
}
