use std::{collections::HashMap, sync::Arc, time::{SystemTime, UNIX_EPOCH}};

use tokio::sync::{mpsc, Mutex};

use crate::{MdError, MdEvent, Quote, QuoteUpdate, ServiceStats, Symbol, SymbolHealth, SymbolStatus};

/// Market-data service baseline.
///
/// This implementation is intentionally not production-quality and not low-latency. It is here to
/// give you something concrete to refactor. Preserve the public methods used by tests, but feel
/// free to replace all internals.
#[derive(Clone)]
pub struct MarketDataService {
    quotes: Arc<Mutex<HashMap<Symbol, Quote>>>,
    status: Arc<Mutex<HashMap<Symbol, SymbolStatus>>>,
    stats: Arc<Mutex<ServiceStats>>,
}

impl Default for MarketDataService {
    fn default() -> Self {
        Self {
            quotes: Arc::new(Mutex::new(HashMap::new())),
            status: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(ServiceStats::default())),
        }
    }
}

impl MarketDataService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Process a stream of events.
    pub async fn run(&self, mut rx: mpsc::Receiver<MdEvent>) -> Result<(), MdError> {
        while let Some(event) = rx.recv().await {
            let service = self.clone();
            tokio::spawn(async move {
                let _ = service.apply_event(event).await;
            });
        }

        Ok(())
    }

    /// Apply one event.
    pub async fn apply_event(&self, event: MdEvent) -> Result<(), MdError> {
        let (is_snapshot, update) = match event {
            MdEvent::Incremental(update) => (false, update),
            MdEvent::Snapshot(update) => (true, update),
        };

        let now_ns = wall_clock_ns();
        let mut duplicate = false;
        let mut applied = false;

        {
            let mut status = self.status.lock().await;
            let symbol_status = status.entry(update.symbol.clone()).or_default();

            if update.seq <= symbol_status.last_seq {
                symbol_status.duplicate_count += 1;
                duplicate = true;
            } else {
                symbol_status.last_seq = update.seq;
                symbol_status.health = SymbolHealth::Live;
                applied = true;
            }
        }

        if applied {
            let mut quotes = self.quotes.lock().await;
            quotes.insert(update.symbol.clone(), quote_from_update(&update, now_ns));
        }

        {
            let mut stats = self.stats.lock().await;
            stats.events_seen += 1;
            if is_snapshot {
                stats.snapshots += 1;
            }
            if duplicate {
                stats.duplicates += 1;
            }
            if applied {
                stats.applied += 1;
            }
        }

        Ok(())
    }

    pub async fn get_quote(&self, symbol: impl Into<Symbol>) -> Option<Quote> {
        let symbol = symbol.into();
        let quotes = self.quotes.lock().await;
        quotes.get(&symbol).copied()
    }

    pub async fn symbol_status(&self, symbol: impl Into<Symbol>) -> SymbolStatus {
        let symbol = symbol.into();
        let status = self.status.lock().await;
        status.get(&symbol).copied().unwrap_or_default()
    }

    /// Return a full snapshot.
    pub async fn snapshot(&self) -> HashMap<Symbol, Quote> {
        let quotes = self.quotes.lock().await;
        quotes.clone()
    }

    pub async fn stats(&self) -> ServiceStats {
        *self.stats.lock().await
    }
}

fn quote_from_update(update: &QuoteUpdate, updated_at_ns: u64) -> Quote {
    Quote {
        bid_px: update.bid_px,
        bid_qty: update.bid_qty,
        ask_px: update.ask_px,
        ask_qty: update.ask_qty,
        seq: update.seq,
        updated_at_ns,
    }
}

fn wall_clock_ns() -> u64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    nanos.min(u64::MAX as u128) as u64
}
