//! # Nineties Core
//!
//! Event sourcing primitives for the nineties framework.
//! This crate is headless and contains no web dependencies.
//!
//! ## Features
//!
//! - Event store trait definitions
//! - Aggregate trait definitions
//! - Command and event bus traits
//! - Projection engine traits
//!

// Re-export commonly used types
pub use serde::{Deserialize, Serialize};
pub use uuid::Uuid;

// Module structure
pub mod aggregate;
pub mod command_bus;
pub mod event;
pub mod event_bus;
pub mod event_store;
pub mod projection;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
