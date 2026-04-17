mod backend;
mod codec;
mod contract;
mod mapping;

/// Outbound adapter implementations selected at composition root.
pub mod adapters {
    pub use crate::backend::{InMemoryStorage, PostgresStorage};
}

/// Port contracts consumed by the application layer.
pub mod ports {
    pub use crate::contract::Storage;
}

pub use ports::Storage;
