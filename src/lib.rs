//! Your task is to preserve the public contract used by the tests while replacing the internals.

mod model;
mod service;

pub use model::{
    MdError, MdEvent, Quote, QuoteUpdate, ServiceStats, Symbol, SymbolHealth, SymbolStatus,
};
pub use service::MarketDataService;
