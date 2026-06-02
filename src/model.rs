use std::fmt;

/// External/wire-level symbol representation.
///
/// This implementation is intentionally not production-quality and not low-latency. It is here to
/// give you something concrete to refactor. Preserve the public methods used by tests, but feel
/// free to replace all internals.
#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Symbol(pub String);

impl Symbol {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for Symbol {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for Symbol {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Full top-of-book quote stored by the service.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Quote {
    pub bid_px: i64,
    pub bid_qty: i64,
    pub ask_px: i64,
    pub ask_qty: i64,
    pub seq: u64,
    pub updated_at_ns: u64,
}

/// Incoming full top-of-book quote update.
///
/// In this simplified assignment every message contains a complete top-of-book quote. The service
/// still has to enforce per-symbol sequence semantics: duplicates/stale updates are ignored, gaps
/// are detected, and recovery requires a snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuoteUpdate {
    pub symbol: Symbol,
    pub bid_px: i64,
    pub bid_qty: i64,
    pub ask_px: i64,
    pub ask_qty: i64,
    pub seq: u64,
    pub exchange_ts_ns: u64,
}

impl QuoteUpdate {
    pub fn new(symbol: impl Into<Symbol>, seq: u64, bid_px: i64, ask_px: i64) -> Self {
        Self {
            symbol: symbol.into(),
            bid_px,
            bid_qty: 1,
            ask_px,
            ask_qty: 1,
            seq,
            exchange_ts_ns: 0,
        }
    }

    pub fn with_qty(mut self, bid_qty: i64, ask_qty: i64) -> Self {
        self.bid_qty = bid_qty;
        self.ask_qty = ask_qty;
        self
    }
}

/// Market-data event.
///
/// `Incremental` must be applied only when `seq == last_seq + 1`, except the first incremental for
/// a previously unseen symbol, which may establish initial state.
///
/// `Snapshot` is an explicit recovery boundary: it may replace the current state, clear stale
/// status, and set `last_seq = snapshot.seq`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MdEvent {
    Incremental(QuoteUpdate),
    Snapshot(QuoteUpdate),
}

impl MdEvent {
    pub fn symbol(&self) -> &Symbol {
        match self {
            Self::Incremental(update) | Self::Snapshot(update) => &update.symbol,
        }
    }

    pub fn seq(&self) -> u64 {
        match self {
            Self::Incremental(update) | Self::Snapshot(update) => update.seq,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolHealth {
    /// No event has been applied for this symbol yet.
    Unknown,
    /// Latest state is usable.
    Live,
    Stale,
}

impl Default for SymbolHealth {
    fn default() -> Self {
        Self::Unknown
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SymbolStatus {
    pub health: SymbolHealth,
    pub last_seq: u64,
    pub duplicate_count: u64,
    pub gap_count: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ServiceStats {
    pub events_seen: u64,
    pub applied: u64,
    pub duplicates: u64,
    pub gaps: u64,
    pub snapshots: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MdError {
    ChannelClosed,
}

impl fmt::Display for MdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChannelClosed => f.write_str("market-data channel is closed"),
        }
    }
}

impl std::error::Error for MdError {}
