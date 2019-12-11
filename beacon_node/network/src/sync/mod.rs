//! Syncing for lighthouse.
//!
//! Stores the various syncing methods for the beacon chain.
mod manager;
mod message_processor;
mod network_context;
mod range_sync;

pub use message_processor::MessageProcessor;

/// Currently implemented sync methods.
pub enum SyncMethod {
    SimpleSync,
}
