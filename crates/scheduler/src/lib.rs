mod backend;
mod contract;
mod model;

/// Outbound adapter implementations selected at composition root.
pub mod adapters {
    pub use crate::backend::{
        FileBackedScheduler, InMemoryScheduler, PostgresScheduler, RedisScheduler,
    };
}

/// Port contracts consumed by the application layer.
pub mod ports {
    pub use crate::contract::Scheduler;
}

pub use ports::Scheduler;
