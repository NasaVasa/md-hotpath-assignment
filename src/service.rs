use std::{
    collections::HashMap,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use tokio::sync::{mpsc, Mutex};

use crate::{
    MdError, MdEvent, Quote, QuoteUpdate, ServiceStats, Symbol, SymbolHealth, SymbolStatus,
};

/// Market-data service baseline.
///
/// This implementation is intentionally not production-quality and not low-latency. It is here to
/// give you something concrete to refactor. Preserve the public methods used by tests, but feel
/// free to replace all internals.
#[derive(Clone)]
pub struct MarketDataService {
    inner: Arc<Mutex<Inner>>,
}

#[derive(Default)]
struct Inner {
    symbols: HashMap<Symbol, SymbolState>,
    stats: ServiceStats,
}

#[derive(Clone, Copy, Debug, Default)]
struct SymbolState {
    quote: Option<Quote>,
    status: SymbolStatus,
}

impl Default for MarketDataService {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner::default())),
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
            self.apply_event(event).await?;
        }

        Ok(())
    }

    /// Apply one event.
    pub async fn apply_event(&self, event: MdEvent) -> Result<(), MdError> {
        let (is_snapshot, update) = match event {
            MdEvent::Incremental(update) => (false, update),
            MdEvent::Snapshot(update) => (true, update),
        };

        let mut inner = self.inner.lock().await;
        inner.stats.events_seen += 1;

        if is_snapshot {
            inner.stats.snapshots += 1;
        }

        let transition;

        {
            let state = inner.symbols.entry(update.symbol.clone()).or_default();
            transition = classify_event(is_snapshot, update.seq, state);

            match transition {
                EventTransition::Apply => {
                    state.quote = Some(quote_from_update(&update, wall_clock_ns()));
                    state.status.health = SymbolHealth::Live;
                    state.status.last_seq = update.seq;
                }
                EventTransition::Duplicate => {
                    state.status.duplicate_count += 1;
                }
                EventTransition::Gap => {
                    state.status.health = SymbolHealth::Stale;
                    state.status.gap_count += 1;
                }
            }
        }

        match transition {
            EventTransition::Apply => inner.stats.applied += 1,
            EventTransition::Duplicate => inner.stats.duplicates += 1,
            EventTransition::Gap => inner.stats.gaps += 1,
        }

        Ok(())
    }

    pub async fn get_quote(&self, symbol: impl Into<Symbol>) -> Option<Quote> {
        let symbol = symbol.into();
        let inner = self.inner.lock().await;
        inner.symbols.get(&symbol).and_then(|state| state.quote)
    }

    pub async fn symbol_status(&self, symbol: impl Into<Symbol>) -> SymbolStatus {
        let symbol = symbol.into();
        let inner = self.inner.lock().await;
        inner
            .symbols
            .get(&symbol)
            .map(|state| state.status)
            .unwrap_or_default()
    }

    /// Return a full snapshot.
    pub async fn snapshot(&self) -> HashMap<Symbol, Quote> {
        let inner = self.inner.lock().await;
        inner
            .symbols
            .iter()
            .filter_map(|(symbol, state)| state.quote.map(|quote| (symbol.clone(), quote)))
            .collect()
    }

    pub async fn stats(&self) -> ServiceStats {
        self.inner.lock().await.stats
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EventTransition {
    Apply,
    Duplicate,
    Gap,
}

fn classify_event(is_snapshot: bool, seq: u64, state: &SymbolState) -> EventTransition {
    if is_snapshot || state.quote.is_none() {
        return EventTransition::Apply;
    }

    let last_seq = state.status.last_seq;
    if seq <= last_seq {
        return EventTransition::Duplicate;
    }

    if last_seq.checked_add(1) == Some(seq) {
        EventTransition::Apply
    } else {
        EventTransition::Gap
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
