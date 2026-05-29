pub mod find;
pub mod list;
pub mod read;
pub mod test;
pub mod write;

use std::sync::Arc;
use crate::config::Config;
use crate::protocol::{Tool, ToolResult};
use crate::CovenScoutError;
use serde_json::Value;
use tracing::instrument;

/// Dispatch a tool call by name with the given arguments.
#[instrument(skip(config, arguments), fields(tool = %name))]
pub async fn dispatch(
    name: &str,
    arguments: Value,
    config: Arc<Config>,
) -> Result<ToolResult, CovenScoutError> {
    match name {
        "read" => read::handle(arguments, config).await,
        "write" => write::handle(arguments, config).await,
        "list" => list::handle(arguments, config).await,
        "find" => find::handle(arguments, config).await,
        "test" => test::handle(arguments, config).await,
        other => Ok(ToolResult::err(format!("Unknown tool: {other}"))),
    }
}

/// Return the list of all available tools with their schemas.
pub fn tool_list() -> Vec<Tool> {
    vec![
        read::tool_descriptor(),
        write::tool_descriptor(),
        list::tool_descriptor(),
        find::tool_descriptor(),
        test::tool_descriptor(),
    ]
}
