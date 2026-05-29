// coven-scout: A fast, safe MCP server for filesystem and web operations
// Built for the Coven ecosystem.

pub mod config;
pub mod error;
pub mod protocol;
pub mod security;
pub mod server;
pub mod tools;
pub mod utils;

pub use config::Config;
pub use error::CovenScoutError;
