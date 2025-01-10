pub mod resources;
pub mod tools;
pub mod prompts;

pub use resources::*;
pub use tools::*;
pub use prompts::*;

use std::sync::{Arc, RwLock};

/// Common trait for handlers that need to track state
pub(crate) trait HandlerState: Send + Sync {}

/// Common state wrapper for handlers
pub(crate) type SharedState<T> = Arc<RwLock<T>>;
